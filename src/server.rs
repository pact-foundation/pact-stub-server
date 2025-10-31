use std::convert::Infallible;
use std::pin::Pin;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

use anyhow::anyhow;
use futures::executor::block_on;
use futures::future::Future;
use futures::stream::StreamExt;
use futures::task::{Context, Poll};
use http::{StatusCode};
use http_body_util::BodyExt;
use hyper::{Request as HyperRequest};
use hyper::body::{Bytes, Incoming};
use itertools::Itertools;
use maplit::hashmap;
use pact_matching::{CoreMatchingContext, DiffConfig, Mismatch};
use pact_models::generators::GeneratorTestMode;
use pact_models::prelude::*;
use pact_models::prelude::v4::*;
use pact_models::v4::http_parts::{HttpRequest, HttpResponse};
use pact_models::v4::V4InteractionType;
use regex::Regex;
use tower_http::classify::{ServerErrorsAsFailures, SharedClassifier};
use tower_http::trace::{DefaultMakeSpan, Trace, TraceLayer};
use tower_service::Service;
use tracing::{debug, error, info, warn};
use hyper::Response;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder,
};
use tokio::net::TcpListener;
use tower::Layer;

type BoxBody = http_body_util::combinators::BoxBody<Bytes, Infallible>;

use crate::{pact_support, PactSource};

#[derive(Clone)]
pub struct ServerHandler {
  shared: Arc<Shared>,
}

pub struct Shared {
  sources: Vec<(V4Pact, PactSource)>,
  shared_sources: Option<Arc<Mutex<Vec<(V4Pact, PactSource)>>>>,
  reload_tx: Option<broadcast::Sender<()>>,
  auto_cors: bool,
  cors_referer: bool,
  provider_state: Option<Regex>,
  provider_state_header_name: Option<String>,
  empty_provider_states: bool
}

pub trait TraceLayerProvider {
  fn create_trace_layer() -> TraceLayer<SharedClassifier<ServerErrorsAsFailures>> {
    TraceLayer::new_for_http()
      .make_span_with(DefaultMakeSpan::new().include_headers(true))
  }
}

#[derive(Clone)]
pub struct ServerHandlerLayer;

impl TraceLayerProvider for ServerHandlerLayer {}

impl<S> Layer<S> for ServerHandlerLayer {
  type Service = Trace<S, SharedClassifier<ServerErrorsAsFailures>>;

  fn layer(&self, inner: S) -> Self::Service {
    Self::create_trace_layer().layer(inner)
  }
}

impl ServerHandler {
  pub fn new(
    sources: Vec<(V4Pact, PactSource)>,
    auto_cors: bool,
    cors_referer: bool,
    provider_state: Option<Regex>,
    provider_state_header_name: Option<String>,
    empty_provider_states: bool
  ) -> ServerHandler {
    ServerHandler {
      shared: Arc::new(Shared {
        sources,
        shared_sources: None,
        reload_tx: None,
        auto_cors,
        cors_referer,
        provider_state,
        provider_state_header_name,
        empty_provider_states
      })
    }
  }
  
  pub fn new_with_watch(
    shared_sources: Arc<Mutex<Vec<(V4Pact, PactSource)>>>,
    reload_tx: broadcast::Sender<()>,
    auto_cors: bool,
    cors_referer: bool,
    provider_state: Option<Regex>,
    provider_state_header_name: Option<String>,
    empty_provider_states: bool
  ) -> ServerHandler {
    ServerHandler {
      shared: Arc::new(Shared {
        sources: vec![], // Empty since we use shared_sources
        shared_sources: Some(shared_sources),
        reload_tx: Some(reload_tx),
        auto_cors,
        cors_referer,
        provider_state,
        provider_state_header_name,
        empty_provider_states
      })
    }
  }

  pub fn start_server(self, port: u16) -> Result<(), ExitCode> {
    let addr = std::net::SocketAddrV4::new(std::net::Ipv4Addr::new(0, 0, 0, 0), port);
    let addr = std::net::SocketAddr::V4(addr);

    let handler = self.clone();
    
    // Start reload listener if in watch mode
    if let Some(reload_tx) = &handler.shared.reload_tx {
      let mut reload_rx = reload_tx.subscribe();
      tokio::spawn(async move {
        while let Ok(_) = reload_rx.recv().await {
          info!("Pacts reloaded - server will use updated pacts for new requests");
        }
      });
    }
    
    block_on(async move {
      let listener = match TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(err) => {
          error!("tcp listener failed to bind address: {}", err);
          return Err(ExitCode::FAILURE);
        }
      };

      let local_addr = match listener.local_addr() {
        Ok(a) => a,
        Err(err) => {
          error!("failed to get local address: {}", err);
          return Err(ExitCode::FAILURE);
        }
      };
      info!("Server started on port {}", local_addr.port());

      loop {
        match listener.accept().await {
          Ok((stream, _)) => {
            let handler = handler.clone();
            tokio::spawn(async move {
              let io = TokioIo::new(stream);
              let handler = handler.clone();
              let tower_service = tower::ServiceBuilder::new()
                .layer(ServerHandlerLayer)
                .service(handler);
              let hyper_service = hyper_util::service::TowerToHyperService::new(tower_service);
              if let Err(err) = Builder::new(TokioExecutor::new())
                .serve_connection(io, hyper_service)
                .await
              {
                error!("error serving connection: {:?}", err);
              }
            });
          }
          Err(err) => {
            error!("tcp listener failed to accept: {}", err);
            return Err(ExitCode::FAILURE);
          }
        }
      }
    })
  }
}

impl Service<HyperRequest<Incoming>> for ServerHandler {
  type Response = Response<BoxBody>;
  type Error = hyper::http::Error;
  type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

  fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    Poll::Ready(Ok(()))
  }

  fn call(&mut self, req: HyperRequest<Incoming>) -> Self::Future {
    let shared = self.shared.as_ref();
    let auto_cors = shared.auto_cors;
    let cors_referer = shared.cors_referer;
    
    // Use shared sources if in watch mode, otherwise use static sources
    let sources = if let Some(shared_sources) = &shared.shared_sources {
      shared_sources.lock().unwrap().clone()
    } else {
      shared.sources.clone()
    };
    
    let provider_state = shared.provider_state.clone();
    let provider_state_header_name = shared.provider_state_header_name.clone();
    let empty_provider_states = shared.empty_provider_states;

    Box::pin(async move {
      let (parts, body) = req.into_parts();
      let provider_state = match provider_state_header_name {
        Some(name) => {
          let parts_value = &parts;
          let provider_state_header = parts_value.headers.get(name);
          match provider_state_header {
            Some(header) => Some(Regex::new(header.to_str().unwrap()).unwrap()),
            None => provider_state
          }
        },
        None => provider_state
      };

      let bytes = body
        .boxed()
        .collect()
        .await;
      let body = match bytes {
        Ok(contents) => {
          let bytes = contents.to_bytes();
          if bytes.is_empty() {
            OptionalBody::Empty
          } else {
            OptionalBody::Present(bytes, None, None)
          }
        },
        Err(err) => {
          warn!("Failed to read request body: {}", err);
          OptionalBody::Empty
        }
      };
      let request = pact_support::hyper_request_to_pact_request(parts, body);
      let response = handle_request(request, auto_cors, cors_referer,
        sources, provider_state, empty_provider_states).await;
      pact_support::pact_response_to_hyper_response(&response)
    })
  }
}

fn method_supports_payload(request: &HttpRequest) -> bool {
  matches!(request.method.to_uppercase().as_str(), "POST" | "PUT" | "PATCH")
}

async fn find_matching_request(
  request: &HttpRequest,
  auto_cors: bool,
  cors_referer: bool,
  sources: Vec<(V4Pact, PactSource)>,
  provider_state: Option<Regex>,
  empty_provider_states: bool
) -> anyhow::Result<HttpResponse> {
  match &provider_state {
    Some(state) => info!("Filtering interactions by provider state regex '{}'", state),
    None => ()
  }

  // Get a subset of all interactions across all pacts that match the method and path
  let interactions = sources.iter()
    .flat_map(|(source, _)| {
      source.filter_interactions(V4InteractionType::Synchronous_HTTP)
        .iter()
        .map(|i| (i.as_v4_http().unwrap(), source.clone()))
        .collect_vec()
    })
    .filter(|(http, _)| {
      let path_context = CoreMatchingContext::new(DiffConfig::NoUnexpectedKeys,
        &http.request.matching_rules.rules_for_category("path").unwrap_or_default(),
        &hashmap! {}
      );
      pact_matching::match_method(&http.request.method, &request.method).is_ok() &&
        pact_matching::match_path(&http.request.path, &request.path, &path_context).is_ok()
    })
    .filter(|(i, _)| {
      let ps = &i.provider_states;
      match provider_state {
        Some(ref regex) => empty_provider_states && ps.is_empty() ||
          ps.iter().any(|state|
            empty_provider_states && state.name.is_empty() || regex.is_match(state.name.as_str())),
        None => true
      }
    });

  // Match all interactions from the sublist against the incoming request
  let results = futures::stream::iter(interactions)
    .filter_map(|(i, pact)| async move {
      match pact_matching::match_request(i.request.clone(), request.clone(), &pact.boxed(), &i.boxed()).await {
        Ok(result) => {
          let mismatches = result.mismatches();
          let all_matched = mismatches.iter().all(|mismatch|{
            match mismatch {
              Mismatch::MethodMismatch { .. } => false,
              Mismatch::PathMismatch { .. } => false,
              Mismatch::QueryMismatch { .. } => false,
              Mismatch::BodyMismatch { .. } => !(method_supports_payload(request) && request.body.is_present()),
              _ => true
            }
          });
          if all_matched {
            Some((i.clone(), mismatches.clone()))
          } else {
            None
          }
        }
        Err(_) => None
      }
    })
    .collect::<Vec<_>>()
    .await;

  // Find the result with the least number of mismatches
  let match_results = results.iter()
    .sorted_by(|a, b| Ord::cmp(&a.1.len(), &b.1.len()))
    .cloned()
    .collect::<Vec<(SynchronousHttp, Vec<Mismatch>)>>();

  if match_results.len() > 1 {
    warn!("Found more than one pact request for method {} and path '{}', using the first one with the least number of mismatches",
          request.method, request.path);
  }

  match match_results.first() {
    Some((interaction, _)) => Ok(pact_matching::generate_response(&interaction.response,  &GeneratorTestMode::Provider, &hashmap!{}).await),
    None => {
      if auto_cors && request.method.to_uppercase() == "OPTIONS" {
        let origin = if cors_referer {
          match request.headers {
            Some(ref h) => h.iter()
              .find(|kv| kv.0.to_lowercase() == "referer")
              .map(|kv| kv.1.clone().join(", ")).unwrap_or_else(|| "*".to_string()),
            None => "*".to_string()
          }
        } else { "*".to_string() };
        Ok(HttpResponse {
          headers: Some(hashmap!{
            "Access-Control-Allow-Headers".to_string() => vec!["*".to_string()],
            "Access-Control-Allow-Methods".to_string() => vec!["GET, HEAD, POST, PUT, DELETE, CONNECT, OPTIONS, TRACE, PATCH".to_string()],
            "Access-Control-Allow-Origin".to_string() => vec![origin]
          }),
          .. HttpResponse::default()
        })
      } else {
        Err(anyhow!("No matching request found for path {}", request.path))
      }
    }
  }
}

async fn handle_request(
  request: HttpRequest,
  auto_cors: bool,
  cors_referrer: bool,
  sources: Vec<(V4Pact, PactSource)>,
  provider_state: Option<Regex>,
  empty_provider_states: bool
) -> HttpResponse {
  info! ("===> Received {}", request);
  debug!("     body: '{}'", request.body.display_string());
  debug!("     matching_rules: {:?}", request.matching_rules);
  debug!("     generators: {:?}", request.generators);
  match find_matching_request(&request, auto_cors, cors_referrer, sources, provider_state,
                              empty_provider_states).await {
    Ok(response) => response,
    Err(msg) => {
      warn!("{}, sending {}", msg, StatusCode::NOT_FOUND);
      let mut response = HttpResponse {
        status: StatusCode::NOT_FOUND.as_u16(),
        .. HttpResponse::default()
      };
      if auto_cors {
        response.headers = Some(hashmap!{ "Access-Control-Allow-Origin".to_string() => vec!["*".to_string()] })
      }
      response
    }
  }
}

#[cfg(test)]
mod test {
  use expectest::prelude::*;
  use maplit::*;
  use pact_models::matchingrules;
  use pact_models::matchingrules::MatchingRule;
  use pact_models::prelude::*;
  use pact_models::prelude::v4::*;
  use pact_models::v4::http_parts::{HttpRequest, HttpResponse};
  use pact_models::v4::interaction::V4Interaction;
  use regex::Regex;

  use crate::PactSource;

  #[tokio::test]
  async fn match_request_finds_the_most_appropriate_response() {
    let interaction1 = SynchronousHttp::default();
    let interaction2 = SynchronousHttp::default();
    let pact = V4Pact {
      interactions: vec![ interaction1.boxed_v4(), interaction2.boxed_v4() ],
      .. V4Pact::default()
    };

    let request1 = HttpRequest::default();

    expect!(super::find_matching_request(&request1, false, false, vec![(pact, PactSource::Unknown)], None, false).await)
      .to(be_ok().value(interaction1.response));
  }

  #[tokio::test]
  async fn match_request_excludes_requests_with_different_methods() {
    let interaction1 = SynchronousHttp { request: HttpRequest { method: "PUT".to_string(),
        .. HttpRequest::default() }, .. SynchronousHttp::default() };
    let interaction2 = SynchronousHttp::default();
    let pact = V4Pact {
      interactions: vec![ interaction1.boxed_v4(), interaction2.boxed_v4() ],
      .. V4Pact::default()
    };

    let request1 = HttpRequest { method: "POST".to_string(), .. HttpRequest::default() };

    expect!(super::find_matching_request(&request1, false, false, vec![(pact, PactSource::Unknown)], None, false).await)
      .to(be_err());
  }

  #[tokio::test]
  async fn match_request_excludes_requests_with_different_paths() {
    let interaction1 = SynchronousHttp {
      request: HttpRequest { path: "/one".to_string(), .. HttpRequest::default() },
      .. SynchronousHttp::default()
    };

    let interaction2 = SynchronousHttp::default();

    let pact = V4Pact {
      interactions: vec![ interaction1.boxed_v4(), interaction2.boxed_v4() ],
      .. V4Pact::default()
    };

    let request1 = HttpRequest { path: "/two".to_string(), .. HttpRequest::default() };

    expect!(super::find_matching_request(&request1, false, false, vec![(pact, PactSource::Unknown)], None, false).await)
      .to(be_err());
  }

  #[tokio::test]
  async fn match_request_excludes_requests_with_different_query_params() {
    let interaction1 = SynchronousHttp { request: HttpRequest {
        query: Some(hashmap!{ "A".to_string() => vec![ Some("B".to_string()) ] }),
        .. HttpRequest::default() }, .. SynchronousHttp::default() };
    let interaction2 = SynchronousHttp::default();
    let pact = V4Pact {
      interactions: vec![ interaction1.boxed_v4(), interaction2.boxed_v4() ],
      .. V4Pact::default()
    };

    let request1 = HttpRequest {
        query: Some(hashmap!{ "A".to_string() => vec![ Some("C".to_string()) ] }),
        .. HttpRequest::default() };

    expect!(super::find_matching_request(&request1, false, false, vec![(pact, PactSource::Unknown)], None, false).await)
      .to(be_err());
  }

  #[tokio::test]
  async fn match_request_excludes_put_or_post_requests_with_different_bodies() {
    let interaction1 = SynchronousHttp { request: HttpRequest {
        method: "PUT".to_string(),
        body: OptionalBody::Present("{\"a\": 1, \"b\": 2, \"c\": 3}".as_bytes().into(), None, None),
        .. HttpRequest::default() },
        response: HttpResponse { status: 200, .. HttpResponse::default() },
        .. SynchronousHttp::default() };

    let interaction2 = SynchronousHttp { request: HttpRequest {
        method: "PUT".to_string(),
        body: OptionalBody::Present("{\"a\": 2, \"b\": 4, \"c\": 6}".as_bytes().into(), None, None),
        matching_rules: matchingrules!{
            "body" => {
                "$.c" => [ MatchingRule::Integer ]
            }
        },
        .. HttpRequest::default() },
        response: HttpResponse { status: 201, .. HttpResponse::default() },
        .. SynchronousHttp::default() };

    let pact = V4Pact {
      interactions: vec![ interaction1.boxed_v4(), interaction2.boxed_v4() ],
      .. V4Pact::default()
    };

    let request1 = HttpRequest { method: "PUT".to_string(), body: OptionalBody::Present("{\"a\": 1, \"b\": 2, \"c\": 3}".as_bytes().into(), None, None),
        .. HttpRequest::default() };
    let request2 = HttpRequest { method: "PUT".to_string(), body: OptionalBody::Present("{\"a\": 2, \"b\": 5, \"c\": 3}".as_bytes().into(), None, None),
        .. HttpRequest::default() };
    let request3 = HttpRequest { method: "PUT".to_string(), body: OptionalBody::Present("{\"a\": 2, \"b\": 4, \"c\": 16}".as_bytes().into(), None, None),
        .. HttpRequest::default() };
    let request4 = HttpRequest { method: "PUT".to_string(), headers: Some(hashmap!{ "Content-Type".to_string() => vec!["application/json".to_string()] }),
        .. HttpRequest::default() };

    expect!(super::find_matching_request(&request1, false, false, vec![(pact.clone(), PactSource::Unknown)], None, false).await).to(be_ok());
    expect!(super::find_matching_request(&request2, false, false, vec![(pact.clone(), PactSource::Unknown)], None, false).await).to(be_err());
    expect!(super::find_matching_request(&request3, false, false, vec![(pact.clone(), PactSource::Unknown)], None, false).await).to(be_ok());
    expect!(super::find_matching_request(&request4, false, false, vec![(pact, PactSource::Unknown)], None, false).await).to(be_ok());
  }

  #[tokio::test]
  async fn match_request_returns_the_closest_match() {
    let interaction1 = SynchronousHttp { request: HttpRequest {
        body: OptionalBody::Present("{\"a\": 1, \"b\": 2, \"c\": 3}".as_bytes().into(), None, None),
        .. HttpRequest::default() },
        response: HttpResponse { status: 200, .. HttpResponse::default() },
        .. SynchronousHttp::default() };

    let interaction2 = SynchronousHttp { request: HttpRequest {
        body: OptionalBody::Present("{\"a\": 2, \"b\": 4, \"c\": 6}".as_bytes().into(), None, None),
        .. HttpRequest::default() },
        response: HttpResponse { status: 201, .. HttpResponse::default() },
        .. SynchronousHttp::default() };

    let pact1 = V4Pact {
      interactions: vec![ interaction1.boxed_v4() ],
      .. V4Pact::default()
    };
    let pact2 = V4Pact {
      interactions: vec![ interaction2.boxed_v4() ],
      .. V4Pact::default()
    };

    let request1 = HttpRequest {
        body: OptionalBody::Present("{\"a\": 1, \"b\": 4, \"c\": 6}".as_bytes().into(), None, None),
        .. HttpRequest::default() };

    expect!(super::find_matching_request(&request1, false, false, vec![(pact1, PactSource::Unknown), (pact2, PactSource::Unknown)], None, false).await)
      .to(be_ok().value(interaction2.response));
  }

  #[tokio::test]
  async fn with_auto_cors_return_200_with_an_option_request() {
    let interaction1 = SynchronousHttp::default();
    let pact = V4Pact {
      interactions: vec![ interaction1.boxed_v4() ],
      .. V4Pact::default()
    };

    let request1 = HttpRequest {
        method: "OPTIONS".to_string(),
        .. HttpRequest::default() };

    expect!(super::find_matching_request(&request1, true, false, vec![(pact.clone(), PactSource::Unknown)], None, false).await)
      .to(be_ok());
    expect!(super::find_matching_request(&request1, false, false, vec![(pact, PactSource::Unknown)], None, false).await)
      .to(be_err());
  }

  #[tokio::test]
  async fn match_request_with_query_params() {
    let matching_rules = matchingrules!{
        "query" => {
            "page[0]" => [ MatchingRule::Type ]
        }
    };
    let interaction1 = SynchronousHttp {
        request: HttpRequest {
            path: "/api/objects".to_string(),
            query: Some(hashmap!{ "page".to_string() => vec![ Some("1".to_string()) ] }),
            .. HttpRequest::default()
        },
        .. SynchronousHttp::default()
    };

    let interaction2 = SynchronousHttp {
        request: HttpRequest {
            path: "/api/objects".to_string(),
            query: Some(hashmap!{ "page".to_string() => vec![ Some("1".to_string()) ] }),
            matching_rules,
            .. HttpRequest::default()
        },
        .. SynchronousHttp::default()
    };

    let pact = V4Pact {
      interactions: vec![ interaction1.boxed_v4(), interaction2.boxed_v4() ],
      .. V4Pact::default()
    };

    let request1 = HttpRequest {
        path: "/api/objects".to_string(),
        query: Some(hashmap!{ "page".to_string() => vec![ Some("3".to_string()) ] }),
        .. HttpRequest::default() };

    expect!(super::find_matching_request(&request1, false, false, vec![(pact, PactSource::Unknown)], None, false).await)
      .to(be_ok());
  }

  #[test_log::test(tokio::test)]
  async fn match_request_with_repeated_query_params() {
    let matching_rules = matchingrules!{
        "query" => {
            "ids" => [ MatchingRule::MinType(2) ],
            "ids[*]" => [ MatchingRule::Type ]
        }
    };
    let interaction = SynchronousHttp {
      request: HttpRequest {
        path: "/api".to_string(),
        query: Some(hashmap!{
          "ids".to_string() => vec![
            Some("1".to_string()),
            Some("2".to_string()),
            Some("3".to_string()),
            Some("4".to_string())
          ]
        }),
        matching_rules,
        .. HttpRequest::default()
      },
      .. SynchronousHttp::default()
    };

    let pact = V4Pact {
      interactions: vec![ interaction.boxed_v4() ],
      .. V4Pact::default()
    };

    let request1 = HttpRequest {
      path: "/api".to_string(),
      query: Some(hashmap!{ "ids".to_string() => vec![ Some("3".to_string()) ] }),
      .. HttpRequest::default() };
    let request2 = HttpRequest {
      path: "/api".to_string(),
      query: Some(hashmap!{ "ids".to_string() => vec![ Some("3".to_string()), Some("1".to_string()) ] }),
      .. HttpRequest::default() };
    let request3 = HttpRequest {
      path: "/api".to_string(),
      query: Some(hashmap!{ "ids".to_string() => vec![
        Some("1".to_string()),
        Some("2".to_string()),
        Some("3".to_string()),
        Some("4".to_string())
      ] }),
      .. HttpRequest::default() };
    let request4 = HttpRequest {
      path: "/api".to_string(),
      query: Some(hashmap!{ "ids".to_string() => vec![
        Some("id".to_string()),
        Some("id".to_string()),
        Some("id".to_string()),
        Some("id".to_string())
      ] }),
      .. HttpRequest::default() };
    let request5 = HttpRequest {
      path: "/api".to_string(),
      query: Some(hashmap!{ "ids".to_string() => vec![
        Some("1".to_string()),
        Some("2".to_string()),
        Some("3".to_string()),
        Some("4".to_string()),
        Some("5".to_string())
      ] }),
      .. HttpRequest::default() };

    expect!(super::find_matching_request(&request1, false, false, vec![(pact.clone(), PactSource::Unknown)], None, false).await)
      .to(be_err());
    expect!(super::find_matching_request(&request2, false, false, vec![(pact.clone(), PactSource::Unknown)], None, false).await)
      .to(be_ok());
    expect!(super::find_matching_request(&request3, false, false, vec![(pact.clone(), PactSource::Unknown)], None, false).await)
      .to(be_ok());
    expect!(super::find_matching_request(&request4, false, false, vec![(pact.clone(), PactSource::Unknown)], None, false).await)
      .to(be_ok());
    expect!(super::find_matching_request(&request5, false, false, vec![(pact.clone(), PactSource::Unknown)], None, false).await)
      .to(be_ok());
  }

  #[tokio::test]
  async fn match_request_filters_interactions_if_provider_state_filter_is_provided() {
    let response1 = HttpResponse { status: 201, .. HttpResponse::default() };
    let interaction1 = SynchronousHttp {
        provider_states: vec![ ProviderState::default("state one") ],
        request: HttpRequest::default(),
        response: HttpResponse { status: 201, .. HttpResponse::default() },
        .. SynchronousHttp::default() };

    let response2 = HttpResponse { status: 202, .. HttpResponse::default() };
    let interaction2 = SynchronousHttp {
        provider_states: vec![ ProviderState::default("state two") ],
        request: HttpRequest::default(),
        response: HttpResponse { status: 202, .. HttpResponse::default() },
        .. SynchronousHttp::default() };

    let response3 = HttpResponse { status: 203, .. HttpResponse::default() };
    let interaction3 = SynchronousHttp {
        provider_states: vec![ ProviderState::default("state one"),
                               ProviderState::default("state two"),
                               ProviderState::default("state three") ],
        request: HttpRequest::default(),
        response: HttpResponse { status: 203, .. HttpResponse::default() },
        .. SynchronousHttp::default() };
    let interaction4 = SynchronousHttp {
      response: HttpResponse { status: 204, .. HttpResponse::default() },
      .. SynchronousHttp::default() };

    let pact = V4Pact {
      interactions: vec![ interaction1.boxed_v4(), interaction2.boxed_v4(), interaction3.boxed_v4(), interaction4.boxed_v4() ],
      .. V4Pact::default()
    };

    let request = HttpRequest::default();

    expect!(super::find_matching_request(&request, false, false, vec![(pact.clone(), PactSource::Unknown)],
      Some(Regex::new("state one").unwrap()), false).await).to(be_ok().value(response1.clone()));
    expect!(super::find_matching_request(&request, false, false, vec![(pact.clone(), PactSource::Unknown)],
      Some(Regex::new("state two").unwrap()), false).await).to(be_ok().value(response2.clone()));
    expect!(super::find_matching_request(&request, false, false, vec![(pact.clone(), PactSource::Unknown)],
      Some(Regex::new("state three").unwrap()), false).await).to(be_ok().value(response3.clone()));
    expect!(super::find_matching_request(&request, false, false, vec![(pact.clone(), PactSource::Unknown)],
      Some(Regex::new("state four").unwrap()), false).await).to(be_err());
    expect!(super::find_matching_request(&request, false, false, vec![(pact.clone(), PactSource::Unknown)],
      Some(Regex::new("state .*").unwrap()), false).await).to(be_ok().value(response1.clone()));
  }

  #[tokio::test]
  async fn match_request_filters_interactions_if_provider_state_filter_is_provided_and_empty_values_included() {
    let interaction1 = SynchronousHttp {
      provider_states: vec![ ProviderState::default("state one") ],
      request: HttpRequest::default(),
      response: HttpResponse { status: 201, .. HttpResponse::default() },
      .. SynchronousHttp::default() };

    let response2 = HttpResponse { status: 202, .. HttpResponse::default() };
    let interaction2 = SynchronousHttp {
      provider_states: vec![ ProviderState::default("") ],
      request: HttpRequest::default(),
      response: HttpResponse { status: 202, .. HttpResponse::default() },
      .. SynchronousHttp::default() };

    let response3 = HttpResponse { status: 203, .. HttpResponse::default() };
    let interaction3 = SynchronousHttp {
      request: HttpRequest::default(),
      response: HttpResponse { status: 203, .. HttpResponse::default() },
      .. SynchronousHttp::default() };

    let pact1 = V4Pact {
      interactions: vec![ interaction1.boxed_v4(), interaction2.boxed_v4(), interaction3.boxed_v4() ],
      .. V4Pact::default()
    };

    let pact2 = V4Pact {
      interactions: vec![ interaction1.boxed_v4(), interaction3.boxed_v4() ],
      .. V4Pact::default()
    };

    let request = HttpRequest::default();

    expect!(super::find_matching_request(&request, false, false, vec![(pact1, PactSource::Unknown)],
      Some(Regex::new("any state").unwrap()), true).await).to(be_ok().value(response2.clone()));

    expect!(super::find_matching_request(&request, false, false, vec![(pact2, PactSource::Unknown)],
      Some(Regex::new("any state").unwrap()), true).await).to(be_ok().value(response3.clone()));
  }

  #[tokio::test]
  async fn handles_repeated_headers_values() {
    let interaction = SynchronousHttp {
        request: HttpRequest { headers: Some(hashmap!{ "TEST-X".to_string() => vec!["X, Z".to_string()] }),  .. HttpRequest::default() },
        response: HttpResponse { headers: Some(hashmap!{ "TEST-X".to_string() => vec!["X, Y".to_string()] }), .. HttpResponse::default() },
        .. SynchronousHttp::default() };
    let pact = V4Pact {
      interactions: vec![ interaction.boxed_v4() ],
      .. V4Pact::default()
    };

    let request = HttpRequest { headers: Some(hashmap!{ "TEST-X".to_string() => vec!["X, Y".to_string()] }), .. HttpRequest::default() };

    let result = super::find_matching_request(&request, false, false, vec![(pact, PactSource::Unknown)], None, false).await;
    expect!(result).to(be_ok().value(interaction.response));
  }
}
