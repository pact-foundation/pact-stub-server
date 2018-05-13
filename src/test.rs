use quickcheck::{TestResult, quickcheck};
use rand::Rng;
use super::{integer_value, ServerHandler};
use expectest::prelude::*;
use pact_matching::models::{Pact, Interaction, Request, Response, OptionalBody};
use models::matchingrules::*;

#[test]
fn validates_integer_value() {
    fn prop(s: String) -> TestResult {
        let mut rng = ::rand::thread_rng();
        if rng.gen() && s.chars().any(|ch| !ch.is_numeric()) {
            TestResult::discard()
        } else {
            let validation = integer_value(s.clone());
            match validation {
                Ok(_) => TestResult::from_bool(!s.is_empty() && s.chars().all(|ch| ch.is_numeric() )),
                Err(_) => TestResult::from_bool(s.is_empty() || s.chars().find(|ch| !ch.is_numeric() ).is_some())
            }
        }
    }
    quickcheck(prop as fn(_) -> _);

    expect!(integer_value(s!("1234"))).to(be_ok());
    expect!(integer_value(s!("1234x"))).to(be_err());
}

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
