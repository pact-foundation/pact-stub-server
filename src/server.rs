use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use futures::executor::block_on;
use futures::task::{Context, Poll};
use http::{Error, StatusCode};
use hyper::{Body, Request as HyperRequest, Response as HyperResponse, Server};
use itertools::Itertools;
use log::*;
use maplit::*;
use pact_matching::{self, Mismatch, MatchingContext, DiffConfig};
use pact_matching::models::{Interaction, Request, Response, RequestResponseInteraction};
use pact_matching::models::OptionalBody;
use pact_matching::s;
use regex::Regex;
use tower_service::Service;

use crate::pact_support;
use pact_matching::models::generators::GeneratorTestMode;

#[derive(Clone)]
pub struct ServerHandler {
  sources: Vec<Arc<Mutex<dyn Interaction + Send + Sync>>>,
  auto_cors: bool,
  cors_referer: bool,
  provider_state: Option<Regex>,
  provider_state_header_name: Option<String>,
  empty_provider_states: bool
}

impl ServerHandler {
  pub fn new(
    sources: Vec<Arc<Mutex<dyn Interaction + Send + Sync>>>,
    auto_cors: bool,
    cors_referer: bool,
    provider_state: Option<Regex>,
    provider_state_header_name: Option<String>,
    empty_provider_states: bool
  ) ->  ServerHandler {
    ServerHandler {
      sources,
      auto_cors,
      cors_referer,
      provider_state,
      provider_state_header_name,
      empty_provider_states
    }
  }

  pub fn start_server(self, port: u16) -> Result<(), i32> {
    let addr = ([0, 0, 0, 0], port).into();
    match Server::try_bind(&addr) {
      Ok(builder) => {
        let server = builder
          .serve(hyper::service::make_service_fn(|_| {
            let inner = self.clone();
            async {
              Ok::<_, hyper::Error>(inner)
            }
          }));
        info!("Server started on port {}", server.local_addr().port());
        block_on(server).map_err(|err| {
          error!("error occurred scheduling server future on Tokio runtime: {}", err);
          2
        })?;
        Ok(())
      },
      Err(err) => {
        error!("could not start server: {}", err);
        Err(1)
      }
    }
  }
}

impl Service<HyperRequest<Body>> for ServerHandler {
  type Response = HyperResponse<Body>;
  type Error = Error;
  type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

  fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    Poll::Ready(Ok(()))
  }

  fn call(&mut self, req: HyperRequest<Body>) -> Self::Future {
    let auto_cors = self.auto_cors.clone();
    let cors_referrer = self.cors_referer;
    let sources = self.sources.clone();
    let provider_state = self.provider_state.clone();
    let provider_state_header_name = self.provider_state_header_name.clone();
    let empty_provider_states = self.empty_provider_states;

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

      let bytes = hyper::body::to_bytes(body).await;
      let body = match bytes {
        Ok(contents) => if contents.is_empty() {
          OptionalBody::Empty
        } else {
          OptionalBody::Present(contents.to_vec(), None)
        },
        Err(err) => {
          warn!("Failed to read request body: {}", err);
          OptionalBody::Empty
        }
      };
      let request = pact_support::hyper_request_to_pact_request(parts, body);
      let response = handle_request(request, auto_cors, cors_referrer, sources, provider_state,
        empty_provider_states);
      pact_support::pact_response_to_hyper_response(&response)
    })
  }
}

fn method_supports_payload(request: &Request) -> bool {
  match request.method.to_uppercase().as_str() {
    "POST" | "PUT" | "PATCH" => true,
    _ => false
  }
}

fn find_matching_request(
  request: &Request,
  auto_cors: bool,
  cors_referer: bool,
  sources: &Vec<RequestResponseInteraction>,
  provider_state: Option<Regex>,
  empty_provider_states: bool
) -> Result<Response, String> {
  match &provider_state {
    Some(state) => info!("Filtering interactions by provider state regex '{}'", state),
    None => ()
  }
  let match_results = sources
    .iter()
    .filter(|interaction| {
      let path_context = MatchingContext::new(DiffConfig::NoUnexpectedKeys,
        &interaction.request.matching_rules.rules_for_category("path").unwrap_or_default());
      pact_matching::match_method(&interaction.request.method, &request.method).is_ok() &&
        pact_matching::match_path(&interaction.request.path, &request.path, &path_context).is_ok()
    })
    .filter(|i| {
      match provider_state {
        Some(ref regex) => empty_provider_states && i.provider_states().is_empty() ||
          i.provider_states().iter().any(|state|
            empty_provider_states && state.name.is_empty() || regex.is_match(state.name.as_str())),
        None => true
      }
    })
    .map(|i| {
      (i.clone(), pact_matching::match_request(i.request.clone(), request.clone()).mismatches())
    })
    .filter(|(_, mismatches)| {
      mismatches.iter().all(|mismatch|{
        match mismatch {
          &Mismatch::MethodMismatch { .. } => false,
          &Mismatch::PathMismatch { .. } => false,
          &Mismatch::QueryMismatch { .. } => false,
          &Mismatch::BodyMismatch { .. } => !(method_supports_payload(request) && request.body.is_present()),
          _ => true
        }
      })
    })
    .sorted_by(|a, b| Ord::cmp(&a.1.len(), &b.1.len()))
    .collect::<Vec<(RequestResponseInteraction, Vec<Mismatch>)>>();

  if match_results.len() > 1 {
    warn!("Found more than one pact request for method {} and path '{}', using the first one with the least number of mismatches",
          request.method, request.path);
  }

  match match_results.first() {
    Some((interaction, _)) => Ok(pact_matching::generate_response(&interaction.response,  &GeneratorTestMode::Provider, &hashmap!{})),
    None => {
      if auto_cors && request.method.to_uppercase() == "OPTIONS" {
        let origin = if cors_referer {
          match request.headers {
            Some(ref h) => h.iter()
              .find(|kv| kv.0.to_lowercase() == "referer")
              .map(|kv| kv.1.clone().join(", ")).unwrap_or("*".to_string()),
            None => "*".to_string()
          }
        } else { "*".to_string() };
        Ok(Response {
          headers: Some(hashmap!{
                    s!("Access-Control-Allow-Headers") => vec![s!("*")],
                    s!("Access-Control-Allow-Methods") => vec![s!("GET, HEAD, POST, PUT, DELETE, CONNECT, OPTIONS, TRACE, PATCH")],
                    s!("Access-Control-Allow-Origin") => vec![origin]
                  }),
          .. Response::default()
        })
      } else {
        Err(s!("No matching request found"))
      }
    }
  }
}

fn handle_request(
  request: Request,
  auto_cors: bool,
  cors_referrer: bool,
  sources: Vec<Arc<Mutex<dyn Interaction + Send + Sync>>>,
  provider_state: Option<Regex>,
  empty_provider_states: bool
) -> Response {
  info! ("===> Received {}", request);
  debug!("     body: '{}'", request.body.str_value());
  debug!("     matching_rules: {:?}", request.matching_rules);
  debug!("     generators: {:?}", request.generators);
  let interactions = sources.iter().map(|i| {
    let guard = i.lock().unwrap();
    guard.as_request_response()
  }).filter(|i| i.is_some())
    .map(|i| i.unwrap())
    .collect();
  match find_matching_request(&request, auto_cors, cors_referrer, &interactions, provider_state,
                              empty_provider_states) {
    Ok(response) => response,
    Err(msg) => {
      warn!("{}, sending {}", msg, StatusCode::NOT_FOUND);
      let mut response = Response {
        status: StatusCode::NOT_FOUND.as_u16(),
        .. Response::default()
      };
      if auto_cors {
        response.headers = Some(hashmap!{ s!("Access-Control-Allow-Origin") => vec![s!("*")] })
      }
      response
    }
  }
}

#[cfg(test)]
mod test {
  use expectest::prelude::*;
  use maplit::*;
  use pact_matching::{matchingrules, s};
  use pact_matching::models::{OptionalBody, Request, RequestResponseInteraction, Response};
  use pact_matching::models::matchingrules::*;
  use pact_matching::models::provider_states::*;
  use regex::Regex;

  #[test]
    fn match_request_finds_the_most_appropriate_response() {
      let interaction1 = RequestResponseInteraction::default();
      let interaction2 = RequestResponseInteraction::default();

      let request1 = Request::default();

      expect!(super::find_matching_request(&request1, false, false,
        &vec![interaction1.clone(), interaction2.clone()], None, false)).to(
        be_ok().value(interaction1.response));
    }

    #[test]
    fn match_request_excludes_requests_with_different_methods() {
      let interaction1 = RequestResponseInteraction { request: Request { method: s!("PUT"),
          .. Request::default() }, .. RequestResponseInteraction::default() };

      let interaction2 = RequestResponseInteraction::default();

      let request1 = Request { method: s!("POST"), .. Request::default() };

      expect!(super::find_matching_request(&request1, false, false,
        &vec![interaction1, interaction2], None, false)).to(be_err());
    }

    #[test]
    fn match_request_excludes_requests_with_different_paths() {
      let interaction1 = RequestResponseInteraction {
        request: Request { path: s!("/one"), .. Request::default() },
        .. RequestResponseInteraction::default()
      };

      let interaction2 = RequestResponseInteraction::default();

      let request1 = Request { path: s!("/two"), .. Request::default() };

      expect!(super::find_matching_request(&request1, false, false,
        &vec![interaction1, interaction2], None, false)).to(be_err());
    }

    #[test]
    fn match_request_excludes_requests_with_different_query_params() {
      let interaction1 = RequestResponseInteraction { request: Request {
          query: Some(hashmap!{ s!("A") => vec![ s!("B") ] }),
          .. Request::default() }, .. RequestResponseInteraction::default() };

      let interaction2 = RequestResponseInteraction::default();

      let request1 = Request {
          query: Some(hashmap!{ s!("A") => vec![ s!("C") ] }),
          .. Request::default() };

      expect!(super::find_matching_request(&request1, false, false,
        &vec![interaction1, interaction2], None, false)).to(be_err());
    }

    #[test]
    fn match_request_excludes_put_or_post_requests_with_different_bodies() {
      let interaction1 = RequestResponseInteraction { request: Request {
          method: s!("PUT"),
          body: OptionalBody::Present("{\"a\": 1, \"b\": 2, \"c\": 3}".as_bytes().into(), None),
          .. Request::default() },
          response: Response { status: 200, .. Response::default() },
          .. RequestResponseInteraction::default() };

      let interaction2 = RequestResponseInteraction { request: Request {
          method: s!("PUT"),
          body: OptionalBody::Present("{\"a\": 2, \"b\": 4, \"c\": 6}".as_bytes().into(), None),
          matching_rules: matchingrules!{
              "body" => {
                  "$.c" => [ MatchingRule::Integer ]
              }
          },
          .. Request::default() },
          response: Response { status: 201, .. Response::default() },
          .. RequestResponseInteraction::default() };

      let request1 = Request { method: s!("PUT"), body: OptionalBody::Present("{\"a\": 1, \"b\": 2, \"c\": 3}".as_bytes().into(), None),
          .. Request::default() };
      let request2 = Request { method: s!("PUT"), body: OptionalBody::Present("{\"a\": 2, \"b\": 5, \"c\": 3}".as_bytes().into(), None),
          .. Request::default() };
      let request3 = Request { method: s!("PUT"), body: OptionalBody::Present("{\"a\": 2, \"b\": 4, \"c\": 16}".as_bytes().into(), None),
          .. Request::default() };
      let request4 = Request { method: s!("PUT"), headers: Some(hashmap!{ s!("Content-Type") => vec![s!("application/json")] }),
          .. Request::default() };

      expect!(super::find_matching_request(&request1, false, false,
        &vec![interaction1.clone(), interaction2.clone()], None, false)).to(be_ok());
      expect!(super::find_matching_request(&request2, false, false,
        &vec![interaction1.clone(), interaction2.clone()], None, false)).to(be_err());
      expect!(super::find_matching_request(&request3, false,
        false, &vec![interaction1.clone(), interaction2.clone()], None, false)).to(be_ok());
      expect!(super::find_matching_request(&request4, false, false,
        &vec![interaction1.clone(), interaction2.clone()], None, false)).to(be_ok());
    }

    #[test]
    fn match_request_returns_the_closest_match() {
      let interaction1 = RequestResponseInteraction { request: Request {
          body: OptionalBody::Present("{\"a\": 1, \"b\": 2, \"c\": 3}".as_bytes().into(), None),
          .. Request::default() },
          response: Response { status: 200, .. Response::default() },
          .. RequestResponseInteraction::default() };

      let interaction2 = RequestResponseInteraction { request: Request {
          body: OptionalBody::Present("{\"a\": 2, \"b\": 4, \"c\": 6}".as_bytes().into(), None),
          .. Request::default() },
          response: Response { status: 201, .. Response::default() },
          .. RequestResponseInteraction::default() };

      let request1 = Request {
          body: OptionalBody::Present("{\"a\": 1, \"b\": 4, \"c\": 6}".as_bytes().into(), None),
          .. Request::default() };

      expect!(super::find_matching_request(&request1, false, false,
        &vec![interaction1.clone(), interaction2.clone()], None, false)).to(be_ok().value(interaction2.response));
    }

    #[test]
    fn with_auto_cors_return_200_with_an_option_request() {
      let interaction1 = RequestResponseInteraction::default();

      let request1 = Request {
          method: s!("OPTIONS"),
          .. Request::default() };

      expect!(super::find_matching_request(&request1, true, false,
        &vec![interaction1.clone()], None, false)).to(be_ok());
      expect!(super::find_matching_request(&request1, false, false,
        &vec![interaction1.clone()], None, false)).to(be_err());
    }

    #[test]
    fn match_request_with_query_params() {
      let matching_rules = matchingrules!{
          "query" => {
              "page[0]" => [ MatchingRule::Type ]
          }
      };
      let interaction1 = RequestResponseInteraction {
          request: Request {
              path: s!("/api/objects"),
              query: Some(hashmap!{ s!("page") => vec![ s!("1") ] }),
              .. Request::default()
          },
          .. RequestResponseInteraction::default()
      };

      let interaction2 = RequestResponseInteraction {
          request: Request {
              path: s!("/api/objects"),
              query: Some(hashmap!{ s!("page") => vec![ s!("1") ] }),
              matching_rules,
              .. Request::default()
          },
          .. RequestResponseInteraction::default()
      };

      let request1 = Request {
          path: s!("/api/objects"),
          query: Some(hashmap!{ s!("page") => vec![ s!("3") ] }),
          .. Request::default() };

      expect!(super::find_matching_request(&request1, false, false,
        &vec![interaction1, interaction2], None, false)).to(be_ok());
    }

    #[test]
    fn match_request_filters_interactions_if_provider_state_filter_is_provided() {
      let response1 = Response { status: 201, .. Response::default() };
      let interaction1 = RequestResponseInteraction {
          provider_states: vec![ ProviderState::default(&"state one".into()) ],
          request: Request::default(),
          response: Response { status: 201, .. Response::default() },
          .. RequestResponseInteraction::default() };

      let response2 = Response { status: 202, .. Response::default() };
      let interaction2 = RequestResponseInteraction {
          provider_states: vec![ ProviderState::default(&"state two".into()) ],
          request: Request::default(),
          response: Response { status: 202, .. Response::default() },
          .. RequestResponseInteraction::default() };

      let response3 = Response { status: 203, .. Response::default() };
      let interaction3 = RequestResponseInteraction {
          provider_states: vec![ ProviderState::default(&"state one".into()),
                                 ProviderState::default(&"state two".into()),
                                 ProviderState::default(&"state three".into()) ],
          request: Request::default(),
          response: Response { status: 203, .. Response::default() },
          .. RequestResponseInteraction::default() };
      let interaction4 = RequestResponseInteraction {
        response: Response { status: 204, .. Response::default() },
        .. RequestResponseInteraction::default() };

      let request = Request::default();

      expect!(super::find_matching_request(&request, false, false,
        &vec![interaction1.clone(), interaction2.clone(), interaction3.clone(), interaction4.clone()],
        Some(Regex::new("state one").unwrap()), false)).to(be_ok().value(response1.clone()));
      expect!(super::find_matching_request(&request, false, false,
        &vec![interaction1.clone(), interaction2.clone(), interaction3.clone(), interaction4.clone()],
        Some(Regex::new("state two").unwrap()), false)).to(be_ok().value(response2.clone()));
      expect!(super::find_matching_request(&request, false, false,
        &vec![interaction1.clone(), interaction2.clone(), interaction3.clone(), interaction4.clone()],
        Some(Regex::new("state three").unwrap()), false)).to(be_ok().value(response3.clone()));
      expect!(super::find_matching_request(&request, false, false,
        &vec![interaction1.clone(), interaction2.clone(), interaction3.clone(), interaction4.clone()],
        Some(Regex::new("state four").unwrap()), false)).to(be_err());
      expect!(super::find_matching_request(&request, false, false,
        &vec![interaction1.clone(), interaction2.clone(), interaction3.clone(), interaction4.clone()],
        Some(Regex::new("state .*").unwrap()), false)).to(be_ok().value(response1.clone()));
    }

    #[test]
    fn match_request_filters_interactions_if_provider_state_filter_is_provided_and_empty_values_included() {
      let interaction1 = RequestResponseInteraction {
        provider_states: vec![ ProviderState::default(&"state one".into()) ],
        request: Request::default(),
        response: Response { status: 201, .. Response::default() },
        .. RequestResponseInteraction::default() };

      let response2 = Response { status: 202, .. Response::default() };
      let interaction2 = RequestResponseInteraction {
        provider_states: vec![ ProviderState::default(&"".into()) ],
        request: Request::default(),
        response: Response { status: 202, .. Response::default() },
        .. RequestResponseInteraction::default() };

      let response3 = Response { status: 203, .. Response::default() };
      let interaction3 = RequestResponseInteraction {
        request: Request::default(),
        response: Response { status: 203, .. Response::default() },
        .. RequestResponseInteraction::default() };

      let request = Request::default();

      expect!(super::find_matching_request(&request, false, false,
        &vec![interaction1.clone(), interaction2.clone(), interaction3.clone()],
        Some(Regex::new("any state").unwrap()), true)).to(be_ok().value(response2.clone()));

      expect!(super::find_matching_request(&request, false, false,
        &vec![interaction1.clone(), interaction3.clone()],
        Some(Regex::new("any state").unwrap()), true)).to(be_ok().value(response3.clone()));
    }

    #[test]
    fn handles_repeated_headers_values() {
      let interaction = RequestResponseInteraction {
          request: Request { headers: Some(hashmap!{ s!("TEST-X") => vec![s!("X, Z")] }),  .. Request::default() },
          response: Response { headers: Some(hashmap!{ s!("TEST-X") => vec![s!("X, Y")] }), .. Response::default() },
          .. RequestResponseInteraction::default() };

      let request = Request { headers: Some(hashmap!{ s!("TEST-X") => vec![s!("X, Y")] }), .. Request::default() };

      let result = super::find_matching_request(&request, false, false,
                                                &vec![interaction.clone()], None, false);
      expect!(result).to(be_ok().value(interaction.response));
    }
}
