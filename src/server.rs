use std::sync::Arc;
use hyper::{Body, Request as HyperRequest, Response as HyperResponse, Server};
use hyper::service::service_fn_ok;
use hyper::rt::Future;
use http::StatusCode;
use tokio::runtime::Runtime;
use pact_matching::{self, Mismatch};
use pact_matching::models::{Interaction, Pact, Request, Response};
use pact_support;
use itertools::Itertools;

pub struct ServerHandler {
  sources: Arc<Vec<Pact>>,
  auto_cors: bool
}

fn method_supports_payload(request: &Request) -> bool {
  match request.method.to_uppercase().as_str() {
    "POST" | "PUT" | "PATCH" => true,
    _ => false
  }
}

impl ServerHandler {
    pub fn new(sources: Vec<Pact>, auto_cors: bool) -> ServerHandler {
        ServerHandler {
          sources: Arc::new(sources),
          auto_cors
        }
    }

    pub fn find_matching_request(&self, request: &Request) -> Result<Response, String> {
        let match_results = self.sources
          .iter()
          .flat_map(|pact| pact.interactions.clone())
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
            warn!("Found more than one pact request for method {} and path '{}', using the first one",
                request.method, request.path);
        }

        match match_results.first() {
            Some(interaction) => Ok(pact_matching::generate_response(&interaction.response)),
            None => {
              if self.auto_cors && request.method.to_uppercase() == "OPTIONS" {
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

    fn handle(&self, mut req: HyperRequest<Body>) -> HyperResponse<Body> {
        let request = pact_support::hyper_request_to_pact_request(&mut req);
        info!("\n===> Received request: {:?}", request);
        info!("                   body: '{}'\n", request.body.str_value());
        match self.find_matching_request(&request) {
            Ok(ref response) => pact_support::pact_response_to_hyper_response(response),
            Err(msg) => {
                warn!("{}, sending {}", msg, StatusCode::NOT_FOUND);
                let mut builder = HyperResponse::builder();
                builder.status(StatusCode::NOT_FOUND);
                if self.auto_cors {
                    builder.header("Access-Control-Allow-Origin", "*");
                }
                builder.body(Body::empty()).unwrap()
            }
        }
    }
}

pub fn start_server(port: u16, sources: Vec<Pact>, auto_cors: bool, runtime: &mut Runtime) -> Result<(), i32> {
    let addr = ([0, 0, 0, 0], port).into();
    match Server::try_bind(&addr) {
        Ok(builder) => {
            let server = builder.http1_keepalive(false)
                .serve(move || {
                    let service_handler = ServerHandler::new(sources.clone(), auto_cors);
                    service_fn_ok(move |req| service_handler.handle(req))
                });
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

    use super::ServerHandler;
    use expectest::prelude::*;
    use pact_matching::models::{Pact, Interaction, Request, Response, OptionalBody};
    use pact_matching::models::matchingrules::*;

    #[test]
    fn match_request_finds_the_most_appropriate_response() {
        let interaction1 = Interaction::default();

        let interaction2 = Interaction::default();

        let pact1 = Pact { interactions: vec![ interaction1.clone() ], .. Pact::default() };
        let pact2 = Pact { interactions: vec![ interaction2 ], .. Pact::default() };
        let handler = ServerHandler::new(vec![pact1, pact2], false);

        let request1 = Request::default_request();

        expect!(handler.find_matching_request(&request1)).to(be_ok().value(interaction1.response));
    }

    #[test]
    fn match_request_excludes_requests_with_different_methods() {
        let interaction1 = Interaction { request: Request { method: s!("PUT"),
            .. Request::default_request() }, .. Interaction::default() };

        let interaction2 = Interaction { .. Interaction::default() };

        let pact1 = Pact { interactions: vec![ interaction1 ], .. Pact::default() };
        let pact2 = Pact { interactions: vec![ interaction2 ], .. Pact::default() };
        let handler = ServerHandler::new(vec![pact1, pact2], false);

        let request1 = Request { method: s!("POST"), .. Request::default_request() };

        expect!(handler.find_matching_request(&request1)).to(be_err());
    }

    #[test]
    fn match_request_excludes_requests_with_different_paths() {
        let interaction1 = Interaction { request: Request { path: s!("/one"), .. Request::default_request() }, .. Interaction::default() };

        let interaction2 = Interaction { .. Interaction::default() };

        let pact1 = Pact { interactions: vec![ interaction1 ], .. Pact::default() };
        let pact2 = Pact { interactions: vec![ interaction2 ], .. Pact::default() };
        let handler = ServerHandler::new(vec![pact1, pact2], false);

        let request1 = Request { path: s!("/two"), .. Request::default_request() };

        expect!(handler.find_matching_request(&request1)).to(be_err());
    }

    #[test]
    fn match_request_excludes_requests_with_different_query_params() {
        let interaction1 = Interaction { request: Request {
            query: Some(hashmap!{ s!("A") => vec![ s!("B") ] }),
            .. Request::default_request() }, .. Interaction::default() };

        let interaction2 = Interaction { .. Interaction::default() };

        let pact1 = Pact { interactions: vec![ interaction1 ], .. Pact::default() };
        let pact2 = Pact { interactions: vec![ interaction2 ], .. Pact::default() };
        let handler = ServerHandler::new(vec![pact1, pact2], false);

        let request1 = Request {
            query: Some(hashmap!{ s!("A") => vec![ s!("C") ] }),
            .. Request::default_request() };

        expect!(handler.find_matching_request(&request1)).to(be_err());
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
        let handler = ServerHandler::new(vec![pact1, pact2], false);

        let request1 = Request { method: s!("PUT"), body: OptionalBody::Present("{\"a\": 1, \"b\": 2, \"c\": 3}".as_bytes().into()),
            .. Request::default_request() };
        let request2 = Request { method: s!("PUT"), body: OptionalBody::Present("{\"a\": 2, \"b\": 5, \"c\": 3}".as_bytes().into()),
            .. Request::default_request() };
        let request3 = Request { method: s!("PUT"), body: OptionalBody::Present("{\"a\": 2, \"b\": 4, \"c\": 16}".as_bytes().into()),
            .. Request::default_request() };
        let request4 = Request { method: s!("PUT"), headers: Some(hashmap!{ s!("Content-Type") => s!("application/json") }),
            .. Request::default_request() };

        expect!(handler.find_matching_request(&request1)).to(be_ok());
        expect!(handler.find_matching_request(&request2)).to(be_err());
        expect!(handler.find_matching_request(&request3)).to(be_ok());
        expect!(handler.find_matching_request(&request4)).to(be_ok());
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
        let handler = ServerHandler::new(vec![pact1, pact2], false);

        let request1 = Request {
            body: OptionalBody::Present("{\"a\": 1, \"b\": 4, \"c\": 6}".as_bytes().into()),
            .. Request::default_request() };

        expect!(handler.find_matching_request(&request1)).to(be_ok().value(interaction2.response));
    }

    #[test]
    fn with_auto_cors_return_200_with_an_option_request() {
        let interaction1 = Interaction::default();
        let pact1 = Pact { interactions: vec![ interaction1 ], .. Pact::default() };
        let handler = ServerHandler::new(vec![pact1.clone()], true);
        let handler2 = ServerHandler::new(vec![pact1.clone()], false);

        let request1 = Request {
            method: s!("OPTIONS"),
            .. Request::default_request() };

        expect!(handler.find_matching_request(&request1)).to(be_ok());
        expect!(handler2.find_matching_request(&request1)).to(be_err());
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
        let handler = ServerHandler::new(vec![pact1, pact2], false);

        let request1 = Request {
            path: s!("/api/objects"),
            query: Some(hashmap!{ s!("page") => vec![ s!("3") ] }),
            .. Request::default_request() };

        expect!(handler.find_matching_request(&request1)).to(be_ok());
    }

}
