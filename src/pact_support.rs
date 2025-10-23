use std::collections::HashMap;
use std::convert::Infallible;

use http::{Error, HeaderMap, Uri};
use http::header::{ACCESS_CONTROL_ALLOW_ORIGIN, CONTENT_TYPE};
use http::header::HeaderValue;
use http::request::Parts;
use http_body_util::BodyExt;
use hyper::{Response as HyperResponse};
use hyper::body::Bytes;
use pact_models::content_types::TEXT;
use pact_models::http_parts::HttpPart;
use pact_models::prelude::*;
use pact_models::query_strings::parse_query_string;
use pact_models::v4::http_parts::{HttpRequest, HttpResponse};
use tracing::{debug, info, warn};

type BoxBody = http_body_util::combinators::BoxBody<Bytes, Infallible>;

fn extract_query_string(uri: &Uri) -> Option<HashMap<String, Vec<Option<String>>>> {
    match uri.query() {
        Some(q) => parse_query_string(q),
        None => None
    }
}

fn extract_headers(headers: &HeaderMap<HeaderValue>) -> Option<HashMap<String, Vec<String>>> {
  if !headers.is_empty() {
    let result: HashMap<String, Vec<String>> = headers.keys()
      .map(|name| {
        let values = headers.get_all(name);
        let parsed_vals: Vec<Result<String, ()>> = values.iter()
          .map(|val| val.to_str()
            .map(|v| v.to_string())
            .map_err(|err| {
              warn!("Failed to parse HTTP header value: {}", err);
            })
          ).collect();
        (name.as_str().into(), parsed_vals.iter().cloned()
          .filter(|val| val.is_ok())
          .map(|val| val.unwrap_or_default())
          .collect())
      })
      .collect();
    Some(result)
  } else {
    None
  }
}

pub fn hyper_request_to_pact_request(req: Parts, body: OptionalBody) -> HttpRequest {
  HttpRequest {
    method: req.method.to_string(),
    path: req.uri.path().to_string(),
    query: extract_query_string(&req.uri),
    headers: extract_headers(&req.headers),
    body,
    .. HttpRequest::default()
  }
}

pub fn pact_response_to_hyper_response(response: &HttpResponse) -> Result<HyperResponse<BoxBody>, Error>{
  info!("<=== Sending {}", response);
  debug!("     body: '{}'", response.body.display_string());
  debug!("     matching_rules: {:?}", response.matching_rules);
  debug!("     generators: {:?}", response.generators);
  let mut res = HyperResponse::builder().status(response.status);

  if let Some(headers) = &response.headers {
    for (k, v) in headers.clone() {
      for val in v {
        res = res.header(k.as_str(), val);
      }
    }
  }

  let allow_origin = ACCESS_CONTROL_ALLOW_ORIGIN;
  if !response.has_header(allow_origin.as_str()) {
    res = res.header(allow_origin, "*");
  }

  match &response.body {
    OptionalBody::Present(body, content_type, _) => {
      let content_type_header = CONTENT_TYPE;
      if !response.has_header(content_type_header.as_str()) {
        let content_type = content_type.clone()
          .unwrap_or_else(|| response.content_type().unwrap_or_else(|| TEXT.clone()));
        res = res.header(content_type_header, content_type.to_string());
      }
      let body_bytes = Bytes::copy_from_slice(body);
      let box_body = http_body_util::Full::from(body_bytes).boxed();
      res.body(box_body)
          .map_err(|e| Error::from(e))
      },
    _ => {
      let box_body = http_body_util::Full::from(Bytes::new()).boxed();
      res.body(box_body)
      .map_err(|e| Error::from(e))
    }
  }
}

#[cfg(test)]
mod test {
  use expectest::prelude::*;
  use http::header::HeaderValue;
  use http::status::StatusCode;
  use maplit::*;
  use pact_models::prelude::*;

  use super::*;

  #[test]
  fn test_response() {
      let response = HttpResponse {
          status: 201,
          headers: Some(hashmap! {  }),
          .. HttpResponse::default()
      };
      let hyper_response = pact_response_to_hyper_response(&response).unwrap();

      expect!(hyper_response.status()).to(be_equal_to(StatusCode::CREATED));
      expect!(hyper_response.headers().len()).to(be_equal_to(1));
      expect!(hyper_response.headers().get("Access-Control-Allow-Origin")).to(be_some().value(HeaderValue::from_static("*")));
  }

  #[test]
  fn test_response_with_content_type() {
      let response = HttpResponse {
          status: 201,
          headers: Some(hashmap! { "Content-Type".to_string() => vec!["text/dizzy".to_string()] }),
          body: OptionalBody::Present("{\"a\": 1, \"b\": 4, \"c\": 6}".as_bytes().into(), None, None),
          .. HttpResponse::default()
      };
      let hyper_response = pact_response_to_hyper_response(&response).unwrap();

      expect!(hyper_response.status()).to(be_equal_to(StatusCode::CREATED));
      expect!(hyper_response.headers().is_empty()).to(be_false());
      expect!(hyper_response.headers().get("content-type")).to(be_some().value(HeaderValue::from_static("text/dizzy")));
  }

  #[test]
  fn adds_a_content_type_if_there_is_not_one_and_there_is_a_body() {
      let response = HttpResponse {
          body: OptionalBody::Present("{\"a\": 1, \"b\": 4, \"c\": 6}".as_bytes().into(), None, None),
          .. HttpResponse::default()
      };
      let hyper_response = pact_response_to_hyper_response(&response).unwrap();

      expect!(hyper_response.headers().is_empty()).to(be_false());
      expect!(hyper_response.headers().get("content-type")).to(be_some().value(HeaderValue::from_static("application/json")));
  }

  #[test]
  fn only_add_a_cors_origin_header_if_one_has_not_already_been_provided() {
      let response = HttpResponse {
          headers: Some(hashmap! { "Access-Control-Allow-Origin".to_string() => vec!["dodgy.com".to_string()] }),
          .. HttpResponse::default()
      };
      let hyper_response = pact_response_to_hyper_response(&response).unwrap();

      expect!(hyper_response.headers().len()).to(be_equal_to(1));
      expect!(hyper_response.headers().get("Access-Control-Allow-Origin")).to(be_some().value(HeaderValue::from_static("dodgy.com")));
  }
}
