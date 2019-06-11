use http::StatusCode;
use hyper::{Body, Error as HyperError, Request as HyperRequest, Response as HyperResponse, Server};
use hyper::rt::Future;
use hyper::rt::Stream;
use hyper::service::NewService;
use hyper::service::Service;
use itertools::Itertools;
use pact_matching::{self, Mismatch};
use pact_matching::models::{Interaction, Pact, Request, Response};
use pact_matching::models::OptionalBody;
use pact_support;
use std::sync::Arc;
use tokio::prelude::Async;
use tokio::prelude::future;
use tokio::prelude::future::FutureResult;
use tokio::prelude::IntoFuture;
use tokio::runtime::Runtime;
use regex::Regex;

#[derive(Clone)]
pub struct ServerHandler {
    sources: Arc<Vec<Pact>>,
    auto_cors: bool,
    provider_state: Option<Regex>,
    provider_state_header_name: String
}

fn method_supports_payload(request: &Request) -> bool {
    match request.method.to_uppercase().as_str() {
        "POST" | "PUT" | "PATCH" => true,
        _ => false
    }
}

fn find_matching_request(request: &Request, auto_cors: bool, sources: &Vec<Pact>, provider_state: Option<Regex>) -> Result<Response, String> {
    match provider_state.clone() {
        Some(state) => info!("Filtering interactions by provider state regex '{}'", state),
        None => ()
    }
    let match_results = sources
        .iter()
        .flat_map(|pact| pact.interactions.clone())
        .filter(|i| match provider_state {
            Some(ref regex) => i.provider_states.iter().any(|state| regex.is_match(state.name
                .as_str())),
            None => true
        })
        .map(|i| (i.clone(), pact_matching::match_request(i.request, request.clone())))
        .filter(|&(_, ref mismatches)| mismatches.iter().all(|mismatch|{
            match mismatch {
                &Mismatch::MethodMismatch { .. } => false,
                &Mismatch::PathMismatch { .. } => false,
                &Mismatch::QueryMismatch { .. } => false,
                &Mismatch::BodyMismatch { .. } => !(method_supports_payload(request) && request.body.is_present()),
                _ => true
            }
        }))
        .sorted_by(|a, b| Ord::cmp(&a.1.len(), &b.1.len()))
        .iter()
        .map(|&(ref i, _)| i)
        .cloned()
        .collect::<Vec<Interaction>>();

    if match_results.len() > 1 {
        warn!("Found more than one pact request for method {} and path '{}', using the first one with the least number of mismatches",
              request.method, request.path);
    }

    match match_results.first() {
        Some(interaction) => Ok(pact_matching::generate_response(&interaction.response)),
        None => {
            if auto_cors && request.method.to_uppercase() == "OPTIONS" {
                Ok(Response {
                    headers: Some(hashmap!{
                    s!("Access-Control-Allow-Headers") => s!("authorization,Content-Type"),
                    s!("Access-Control-Allow-Methods") => s!("GET, HEAD, POST, PUT, DELETE, CONNECT, OPTIONS, TRACE, PATCH"),
                    s!("Access-Control-Allow-Origin") => s!("*")
                  }),
                    .. Response::default_response()
                })
            } else {
                Err(s!("No matching request found"))
            }
        }
    }
}

fn handle_request(request: Request, auto_cors: bool, sources: Arc<Vec<Pact>>, provider_state: Option<Regex>) -> Response {
    info! ("===> Received {}", request);
    debug!("     body: '{}'", request.body.str_value());
    debug!("     matching_rules: {:?}", request.matching_rules);
    debug!("     generators: {:?}", request.generators);
    match find_matching_request(&request, auto_cors, &sources, provider_state) {
        Ok(response) => response,
        Err(msg) => {
            warn!("{}, sending {}", msg, StatusCode::NOT_FOUND);
            let mut response = Response {
                status: StatusCode::NOT_FOUND.as_u16(),
                .. Response::default_response()
            };
            if auto_cors {
                response.headers = Some(hashmap!{ s!("Access-Control-Allow-Origin") => s!("*") })
            }
            response
        }
    }
}

impl ServerHandler {
    pub fn new(sources: Vec<Pact>, auto_cors: bool, provider_state: Option<Regex>,
               provider_state_header_name: String) ->  ServerHandler {
        ServerHandler {
            sources: Arc::new(sources),
            auto_cors,
            provider_state,
            provider_state_header_name
        }
    }
}

impl Service for ServerHandler {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = HyperError;
    type Future = ServerHandlerFuture;

    // TODO make the parameter name configurable so there are no collisions with the actual server to be stubbed.
    fn call(&mut self, req: HyperRequest<Body>) -> <Self as Service>::Future {
        let auto_cors = self.auto_cors;
        let sources = self.sources.clone();
        let mut provider_state = self.provider_state.clone();
        let (parts, body) = req.into_parts();
        {
            let yeah = &parts;
            let provider_state_header = yeah.headers.get(self.provider_state_header_name.clone());
            if provider_state_header.is_some() {
                provider_state = Some(Regex::new(provider_state_header.unwrap().to_str().unwrap()).unwrap());
            }
        }

        let future = body.concat2()
            .then(|body| future::ok(match body {
                Ok(chunk) => if chunk.is_empty() {
                    OptionalBody::Empty
                } else {
                    OptionalBody::Present(chunk.iter().cloned().collect())
                },
                Err(err) => {
                    warn!("Failed to read request body: {}", err);
                    OptionalBody::Empty
                }
            }))
            .map(move |body| pact_support::hyper_request_to_pact_request(parts, body))
            .map(move |req| handle_request(req, auto_cors, sources, provider_state))
            .map(|res| pact_support::pact_response_to_hyper_response(&res))
            .into_future();
        ServerHandlerFuture { future: Box::new(future) }
    }
}

pub struct ServerHandlerFuture {
    future: Box<Future<Item=HyperResponse<Body>, Error=HyperError> + Send>
}

impl Future for ServerHandlerFuture {
    type Item = HyperResponse<Body>;
    type Error = HyperError;

    fn poll(&mut self) -> Result<Async<<Self as Future>::Item>, <Self as Future>::Error> {
        self.future.poll()
    }
}

impl NewService for ServerHandler {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = HyperError;
    type Service = ServerHandler;
    type Future = FutureResult<ServerHandler, HyperError>;
    type InitError = HyperError;

    fn new_service(&self) -> <Self as NewService>::Future {
        future::ok(self.clone())
    }
}

pub fn start_server(port: u16, sources: Vec<Pact>, auto_cors: bool, provider_state:
Option<Regex>, provider_state_header_name: String, runtime: &mut Runtime) -> Result<(), i32> {
    let addr = ([0, 0, 0, 0], port).into();
    match Server::try_bind(&addr) {
        Ok(builder) => {
            let server = builder.http1_keepalive(false)
                .serve(ServerHandler::new(sources, auto_cors, provider_state, provider_state_header_name));
            info!("Server started on port {}", server.local_addr().port());
            runtime.block_on(server.map_err(|err| error!("could not start server: {}", err)))
                .map_err(|_| {
                    format!("error occurred scheduling server future on Tokio runtime");
                    2
                })
        },
        Err(err) => {
            error!("could not start server: {}", err);
            Err(1)
        }
    }
}

#[cfg(test)]
mod test {
    use expectest::prelude::*;
    use pact_matching::models::{Interaction, OptionalBody, Pact, Request, Response};
    use pact_matching::models::matchingrules::*;
    use pact_matching::models::provider_states::*;
    use regex::Regex;

    #[test]
    fn match_request_finds_the_most_appropriate_response() {
        let interaction1 = Interaction::default();

        let interaction2 = Interaction::default();

        let pact1 = Pact { interactions: vec![ interaction1.clone() ], .. Pact::default() };
        let pact2 = Pact { interactions: vec![ interaction2 ], .. Pact::default() };

        let request1 = Request::default_request();

        expect!(super::find_matching_request(&request1, false, &vec![pact1, pact2], None)).to(be_ok().value(interaction1.response));
    }

    #[test]
    fn match_request_excludes_requests_with_different_methods() {
        let interaction1 = Interaction { request: Request { method: s!("PUT"),
            .. Request::default_request() }, .. Interaction::default() };

        let interaction2 = Interaction { .. Interaction::default() };

        let pact1 = Pact { interactions: vec![ interaction1 ], .. Pact::default() };
        let pact2 = Pact { interactions: vec![ interaction2 ], .. Pact::default() };

        let request1 = Request { method: s!("POST"), .. Request::default_request() };

        expect!(super::find_matching_request(&request1, false, &vec![pact1, pact2], None)).to(be_err());
    }

    #[test]
    fn match_request_excludes_requests_with_different_paths() {
        let interaction1 = Interaction { request: Request { path: s!("/one"), .. Request::default_request() }, .. Interaction::default() };

        let interaction2 = Interaction { .. Interaction::default() };

        let pact1 = Pact { interactions: vec![ interaction1 ], .. Pact::default() };
        let pact2 = Pact { interactions: vec![ interaction2 ], .. Pact::default() };

        let request1 = Request { path: s!("/two"), .. Request::default_request() };

        expect!(super::find_matching_request(&request1, false, &vec![pact1, pact2], None)).to(be_err());
    }

    #[test]
    fn match_request_excludes_requests_with_different_query_params() {
        let interaction1 = Interaction { request: Request {
            query: Some(hashmap!{ s!("A") => vec![ s!("B") ] }),
            .. Request::default_request() }, .. Interaction::default() };

        let interaction2 = Interaction { .. Interaction::default() };

        let pact1 = Pact { interactions: vec![ interaction1 ], .. Pact::default() };
        let pact2 = Pact { interactions: vec![ interaction2 ], .. Pact::default() };

        let request1 = Request {
            query: Some(hashmap!{ s!("A") => vec![ s!("C") ] }),
            .. Request::default_request() };

        expect!(super::find_matching_request(&request1, false, &vec![pact1, pact2], None)).to(be_err());
    }

    #[test]
    fn match_request_excludes_put_or_post_requests_with_different_bodies() {
        let interaction1 = Interaction { request: Request {
            method: s!("PUT"),
            body: OptionalBody::Present("{\"a\": 1, \"b\": 2, \"c\": 3}".as_bytes().into()),
            .. Request::default_request() },
            response: Response { status: 200, .. Response::default_response() },
            .. Interaction::default() };

        let interaction2 = Interaction { request: Request {
            method: s!("PUT"),
            body: OptionalBody::Present("{\"a\": 2, \"b\": 4, \"c\": 6}".as_bytes().into()),
            matching_rules: matchingrules!{
                "body" => {
                    "$.c" => [ MatchingRule::Integer ]
                }
            },
            .. Request::default_request() },
            response: Response { status: 201, .. Response::default_response() },
            .. Interaction::default() };

        let pact1 = Pact { interactions: vec![ interaction1 ], .. Pact::default() };
        let pact2 = Pact { interactions: vec![ interaction2 ], .. Pact::default() };

        let request1 = Request { method: s!("PUT"), body: OptionalBody::Present("{\"a\": 1, \"b\": 2, \"c\": 3}".as_bytes().into()),
            .. Request::default_request() };
        let request2 = Request { method: s!("PUT"), body: OptionalBody::Present("{\"a\": 2, \"b\": 5, \"c\": 3}".as_bytes().into()),
            .. Request::default_request() };
        let request3 = Request { method: s!("PUT"), body: OptionalBody::Present("{\"a\": 2, \"b\": 4, \"c\": 16}".as_bytes().into()),
            .. Request::default_request() };
        let request4 = Request { method: s!("PUT"), headers: Some(hashmap!{ s!("Content-Type") => s!("application/json") }),
            .. Request::default_request() };

        expect!(super::find_matching_request(&request1, false, &vec![pact1.clone(), pact2.clone()], None)).to(be_ok());
        expect!(super::find_matching_request(&request2, false, &vec![pact1.clone(), pact2.clone()], None)).to(be_err());
        expect!(super::find_matching_request(&request3, false, &vec![pact1.clone(), pact2.clone()], None)).to(be_ok());
        expect!(super::find_matching_request(&request4, false, &vec![pact1.clone(), pact2.clone()], None)).to(be_ok());
    }

    #[test]
    fn match_request_returns_the_closest_match() {
        let interaction1 = Interaction { request: Request {
            body: OptionalBody::Present("{\"a\": 1, \"b\": 2, \"c\": 3}".as_bytes().into()),
            .. Request::default_request() },
            response: Response { status: 200, .. Response::default_response() },
            .. Interaction::default() };

        let interaction2 = Interaction { request: Request {
            body: OptionalBody::Present("{\"a\": 2, \"b\": 4, \"c\": 6}".as_bytes().into()),
            .. Request::default_request() },
            response: Response { status: 201, .. Response::default_response() },
            .. Interaction::default() };

        let pact1 = Pact { interactions: vec![ interaction1 ], .. Pact::default() };
        let pact2 = Pact { interactions: vec![ interaction2.clone() ], .. Pact::default() };

        let request1 = Request {
            body: OptionalBody::Present("{\"a\": 1, \"b\": 4, \"c\": 6}".as_bytes().into()),
            .. Request::default_request() };

        expect!(super::find_matching_request(&request1, false, &vec![pact1, pact2], None)).to(be_ok().value(interaction2.response));
    }

    #[test]
    fn with_auto_cors_return_200_with_an_option_request() {
        let interaction1 = Interaction::default();
        let pact1 = Pact { interactions: vec![ interaction1 ], .. Pact::default() };

        let request1 = Request {
            method: s!("OPTIONS"),
            .. Request::default_request() };

        expect!(super::find_matching_request(&request1, true, &vec![pact1.clone()], None)).to(be_ok());
        expect!(super::find_matching_request(&request1, false, &vec![pact1.clone()], None)).to(be_err());
    }

    #[test]
    fn match_request_with_query_params() {
        let matching_rules = matchingrules!{
            "query" => {
                "page[0]" => [ MatchingRule::Type ]
            }
        };
        let interaction1 = Interaction {
            request: Request {
                path: s!("/api/objects"),
                query: Some(hashmap!{ s!("page") => vec![ s!("1") ] }),
                .. Request::default_request()
            },
            .. Interaction::default()
        };

        let interaction2 = Interaction {
            request: Request {
                path: s!("/api/objects"),
                query: Some(hashmap!{ s!("page") => vec![ s!("1") ] }),
                matching_rules,
                .. Request::default_request()
            },
            .. Interaction::default()
        };

        let pact1 = Pact { interactions: vec![ interaction1 ], .. Pact::default() };
        let pact2 = Pact { interactions: vec![ interaction2 ], .. Pact::default() };

        let request1 = Request {
            path: s!("/api/objects"),
            query: Some(hashmap!{ s!("page") => vec![ s!("3") ] }),
            .. Request::default_request() };

        expect!(super::find_matching_request(&request1, false, &vec![pact1, pact2.clone()], None)).to(be_ok());
    }

    #[test]
    fn match_request_filters_interactions_if_provider_state_filter_is_provided() {
        let response1 = Response { status: 201, .. Response::default_response() };
        let interaction1 = Interaction {
            provider_states: vec![ ProviderState::default(&"state one".into()) ],
            request: Request::default_request(),
            response: Response { status: 201, .. Response::default_response() },
            .. Interaction::default() };

        let response2 = Response { status: 202, .. Response::default_response() };
        let interaction2 = Interaction {
            provider_states: vec![ ProviderState::default(&"state two".into()) ],
            request: Request::default_request(),
            response: Response { status: 202, .. Response::default_response() },
            .. Interaction::default() };

        let response3 = Response { status: 203, .. Response::default_response() };
        let interaction3 = Interaction {
            provider_states: vec![ ProviderState::default(&"state one".into()),
                                   ProviderState::default(&"state two".into()),
                                   ProviderState::default(&"state three".into()) ],
            request: Request::default_request(),
            response: Response { status: 203, .. Response::default_response() },
            .. Interaction::default() };

        let pact = Pact { interactions: vec![ interaction1, interaction2, interaction3 ],
            .. Pact::default() };

        let request = Request::default_request();

        expect!(super::find_matching_request(&request, false, &vec![pact.clone()], Some(Regex::new("state one").unwrap()))).to(be_ok().value(response1.clone()));
        expect!(super::find_matching_request(&request, false, &vec![pact.clone()], Some(Regex::new("state two").unwrap()))).to(be_ok().value(response2.clone()));
        expect!(super::find_matching_request(&request, false, &vec![pact.clone()], Some(Regex::new("state three").unwrap()))).to(be_ok().value(response3.clone()));
        expect!(super::find_matching_request(&request, false, &vec![pact.clone()], Some(Regex::new("state four").unwrap()))).to(be_err());
        expect!(super::find_matching_request(&request, false, &vec![pact.clone()], Some(Regex::new("state .*").unwrap()))).to(be_ok().value(response1.clone()));
    }
}
