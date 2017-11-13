use hyper::server::{Request as HyperRequest, Response as HyperResponse};
use hyper::header::{Headers, ContentType, AccessControlAllowOrigin};
use hyper::mime::{Mime, TopLevel, SubLevel, Attr, Value};
use pact_matching::models::{Request, Response, OptionalBody};
use pact_matching::models::parse_query_string;
use hyper::uri::RequestUri;
use hyper::status::StatusCode;
use std::collections::HashMap;
use std::io::Read;

fn extract_path(uri: &RequestUri) -> String {
    match uri {
        &RequestUri::AbsolutePath(ref s) => s!(s.splitn(2, "?").next().unwrap_or("/")),
        &RequestUri::AbsoluteUri(ref url) => s!(url.path()),
        _ => uri.to_string()
    }
}

fn extract_query_string(uri: &RequestUri) -> Option<HashMap<String, Vec<String>>> {
    match uri {
        &RequestUri::AbsolutePath(ref s) => {
            if s.contains("?") {
                match s.splitn(2, "?").last() {
                    Some(q) => parse_query_string(&s!(q)),
                    None => None
                }
            } else {
                None
            }
        },
        &RequestUri::AbsoluteUri(ref url) => match url.query() {
            Some(q) => parse_query_string(&s!(q)),
            None => None
        },
        _ => None
    }
}

fn extract_headers(headers: &Headers) -> Option<HashMap<String, String>> {
    if headers.len() > 0 {
        Some(headers.iter().map(|h| (s!(h.name()), h.value_string()) ).collect())
    } else {
        None
    }
}

fn extract_body(req: &mut HyperRequest) -> OptionalBody {
    let mut buffer = Vec::new();
    match req.read_to_end(&mut buffer) {
        Ok(size) => if size > 0 {
                OptionalBody::Present(buffer)
            } else {
                OptionalBody::Empty
            },
        Err(err) => {
            warn!("Failed to read request body: {}", err);
            OptionalBody::Empty
        }
    }
}

pub fn hyper_request_to_pact_request(req: &mut HyperRequest) -> Request {
    Request {
        method: req.method.to_string(),
        path: extract_path(&req.uri),
        query: extract_query_string(&req.uri),
        headers: extract_headers(&req.headers),
        body: extract_body(req),
        matching_rules: matchingrules!{}
    }
}

pub fn pact_response_to_hyper_response(mut res: HyperResponse, response: &Response) {
    info!("Sending response {:?}", response);
    *res.status_mut() = StatusCode::from_u16(response.status);
    res.headers_mut().set(AccessControlAllowOrigin::Any);
    res.headers_mut().set(
        ContentType(Mime(TopLevel::Application, SubLevel::Json,
                         vec![(Attr::Charset, Value::Utf8)]))
    );
    match response.headers {
        Some(ref headers) => {
            for (k, v) in headers.clone() {
                res.headers_mut().set_raw(k, vec![v.into_bytes()]);
            }
        },
        None => ()
    }

    match response.body {
        OptionalBody::Present(ref body) => {
            res.send(body).unwrap();
        },
        _ => ()
    }
}
