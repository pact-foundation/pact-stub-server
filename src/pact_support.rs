use http::{HeaderMap, Uri};
use http::header::{ACCESS_CONTROL_ALLOW_ORIGIN, CONTENT_TYPE};
use http::header::HeaderValue;
use http::request::Parts;
use hyper::{Body, Response as HyperResponse};
use pact_matching::models::{HttpPart, OptionalBody, Request, Response};
use pact_matching::models::parse_query_string;
use std::collections::HashMap;

fn extract_query_string(uri: &Uri) -> Option<HashMap<String, Vec<String>>> {
    match uri.query() {
        Some(q) => parse_query_string(&s!(q)),
        None => None
    }
}

fn extract_headers(headers: &HeaderMap<HeaderValue>) -> Option<HashMap<String, String>> {
    if headers.len() > 0 {
        Some(headers.iter().map(|(h, v)| (h.as_str().into(), v.to_str().unwrap_or("").to_string())).collect())
    } else {
        None
    }
}

pub fn hyper_request_to_pact_request(req: Parts, body: OptionalBody) -> Request {
    Request {
        method: req.method.to_string(),
        path: req.uri.path().to_string(),
        query: extract_query_string(&req.uri),
        headers: extract_headers(&req.headers),
        body,
        .. Request::default_request()
    }
}

pub fn pact_response_to_hyper_response(response: &Response) -> HyperResponse<Body> {
    info!("<=== Sending response {:?}", response);
    info!("     body '{}'\n\n", response.body.str_value());
    let mut res = HyperResponse::builder();
    {
        res.status(response.status);

        match response.headers {
            Some(ref headers) => {
                for (k, v) in headers.clone() {
                    res.header(k.as_str(), v);
                }
            },
            None => ()
        }

        if !response.has_header(&ACCESS_CONTROL_ALLOW_ORIGIN.as_str().into()) {
            res.header(ACCESS_CONTROL_ALLOW_ORIGIN, "*");
        }

        match response.body {
            OptionalBody::Present(ref body) => {
                if !response.has_header(&CONTENT_TYPE.as_str().into()) {
                    res.header(CONTENT_TYPE, response.content_type());
                }
                res.body(Body::from(body.clone()))
            },
            _ => res.body(Body::empty())
        }.unwrap()
    }
}

#[cfg(test)]
mod test {
    use expectest::prelude::*;
    use http::header::HeaderValue;
    use http::status::StatusCode;
    use pact_matching::models::{OptionalBody, Response};
    use super::*;

    #[test]
    fn test_response() {
        let response = Response {
            status: 201,
            headers: Some(hashmap! {  }),
            .. Response::default_response()
        };
        let hyper_response = pact_response_to_hyper_response(&response);

        expect!(hyper_response.status()).to(be_equal_to(StatusCode::CREATED));
        expect!(hyper_response.headers().len()).to(be_equal_to(1));
        expect!(hyper_response.headers().get("Access-Control-Allow-Origin")).to(be_some().value(HeaderValue::from_static("*")));
    }

    #[test]
    fn test_response_with_content_type() {
        let response = Response {
            status: 201,
            headers: Some(hashmap! { s!("Content-Type") => s!("text/dizzy") }),
            body: OptionalBody::Present("{\"a\": 1, \"b\": 4, \"c\": 6}".as_bytes().into()),
            .. Response::default_response()
        };
        let hyper_response = pact_response_to_hyper_response(&response);

        expect!(hyper_response.status()).to(be_equal_to(StatusCode::CREATED));
        expect!(hyper_response.headers().is_empty()).to(be_false());
        expect!(hyper_response.headers().get("content-type")).to(be_some().value(HeaderValue::from_static("text/dizzy")));
    }

    #[test]
    fn adds_a_content_type_if_there_is_not_one_and_there_is_a_body() {
        let response = Response {
            body: OptionalBody::Present("{\"a\": 1, \"b\": 4, \"c\": 6}".as_bytes().into()),
            .. Response::default_response()
        };
        let hyper_response = pact_response_to_hyper_response(&response);

        expect!(hyper_response.headers().is_empty()).to(be_false());
        expect!(hyper_response.headers().get("content-type")).to(be_some().value(HeaderValue::from_static("application/json")));
    }

    #[test]
    fn only_add_a_cors_origin_header_if_one_has_not_already_been_provided() {
        let response = Response {
            headers: Some(hashmap! { s!("Access-Control-Allow-Origin") => s!("dodgy.com") }),
            .. Response::default_response()
        };
        let hyper_response = pact_response_to_hyper_response(&response);

        expect!(hyper_response.headers().len()).to(be_equal_to(1));
        expect!(hyper_response.headers().get("Access-Control-Allow-Origin")).to(be_some().value(HeaderValue::from_static("dodgy.com")));
    }
}
