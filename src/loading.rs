//! Functions relating to loading Pact files

use std::fmt::{Display, Formatter};
use std::fs;
use std::panic::RefUnwindSafe;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as Base64;
use futures::future::{ready, Ready};
use futures::StreamExt;
use maplit::hashmap;
use pact_models::pact::{load_pact_from_json, read_pact};
use pact_models::prelude::*;
use pact_verifier::pact_broker::HALClientBuilder;
use regex::Regex;
use serde_json::Value;
use tracing::{debug, warn};

use crate::PactSource;

/// Returns true for HTTP status codes that should be retried.
///
/// Returns true for status codes that indicate a transient failure worth retrying:
/// 5xx server errors, 429 Too Many Requests, and 408 Request Timeout.
fn is_retryable(status: reqwest::StatusCode) -> bool {
  status.is_server_error()
    || status == reqwest::StatusCode::TOO_MANY_REQUESTS
    || status == reqwest::StatusCode::REQUEST_TIMEOUT
}

/// Compute the delay before the next retry attempt.
///
/// For 429 responses with a `Retry-After` header: `delay = secs + min(secs / 5, 60)`.
/// For all other retryable responses: exponential back-off of
/// `500 × 2^(attempt − 1)` milliseconds, giving 500 ms, 1 s, 2 s, 4 s, 8 s, …
pub fn compute_retry_delay(
  status: reqwest::StatusCode,
  retry_after: Option<std::time::Duration>,
  attempt: u32,
) -> std::time::Duration {
  if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
    if let Some(base) = retry_after {
      let secs = base.as_secs();
      let extra = std::cmp::min(secs / 5, 60);
      return std::time::Duration::from_secs(secs + extra);
    }
  }
  std::time::Duration::from_millis(500 * 2_u64.pow(attempt.saturating_sub(1)))
}

fn parse_retry_after(response: &reqwest::Response) -> Option<std::time::Duration> {
  let header_value = response
    .headers()
    .get(reqwest::header::RETRY_AFTER)?
    .to_str()
    .ok()?;

  // Try decimal-seconds form first (e.g. "120").
  if let Ok(secs) = header_value.trim().parse::<u64>() {
    return Some(std::time::Duration::from_secs(secs));
  }

  // Fall back to HTTP-date form (e.g. "Fri, 31 Dec 1999 23:59:59 GMT").
  if let Ok(system_time) = httpdate::parse_http_date(header_value) {
    let delay = system_time
      .duration_since(std::time::SystemTime::now())
      .unwrap_or_default();
    return Some(delay);
  }

  None
}

/// Send `request`, retrying up to `retries` times on 5xx or 429 responses.
///
/// When `retries` is 0 the request is sent exactly once and the response is returned
/// immediately, regardless of status code.
pub async fn with_retries(
  retries: u8,
  request: reqwest::RequestBuilder,
) -> Result<reqwest::Response, reqwest::Error> {
  if retries == 0 {
    return request.send().await;
  }

  let mut last_response: Option<reqwest::Response> = None;
  for attempt in 1..=(retries as u32) {
    let req = request.try_clone().expect("request must be cloneable for retries");
    match req.send().await {
      Ok(response) if is_retryable(response.status()) => {
        let retry_after = parse_retry_after(&response);
        let delay = compute_retry_delay(response.status(), retry_after, attempt);
        last_response = Some(response);
        if attempt < retries as u32 {
          tokio::time::sleep(delay).await;
        }
      }
      other => return other,
    }
  }
  // All attempts were retryable; return the last response.
  Ok(last_response.expect("at least one attempt was made"))
}

#[derive(Debug, Clone)]
pub struct PactError {
  message: String,
  path: Option<String>
}

impl PactError {
  fn new(str: String) -> PactError {
    PactError { message: str, path: None }
  }

  fn with_path(&self, path: &Path) -> PactError {
    PactError {
      message: self.message.clone(),
      path: path.to_str().map(|p| p.to_string())
    }
  }
}

impl Display for PactError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match &self.path {
      Some(path) => write!(f, "{} - {}", self.message, path),
      None => write!(f, "{}", self.message)
    }
  }
}

impl From<reqwest::Error> for PactError {
  fn from(err: reqwest::Error) -> Self {
    PactError { message: format!("Request failed: {}", err), path: None }
  }
}

impl From<serde_json::error::Error> for PactError {
  fn from(err: serde_json::error::Error) -> Self {
    PactError { message: format!("Failed to parse JSON body: {}", err), path: None }
  }
}

impl From<std::io::Error> for PactError {
  fn from(err: std::io::Error) -> Self {
    PactError { message: format!("Failed to load pact file: {}", err), path: None }
  }
}

impl From<anyhow::Error> for PactError {
  fn from(err: anyhow::Error) -> Self {
    PactError { message: format!("Failed to load pact file: {}", err), path: None }
  }
}

fn walkdir(
  dir: &Path,
  ext: &str,
  s: &PactSource
) -> Result<Vec<Result<(Box<dyn Pact + Send + Sync + RefUnwindSafe>, PactSource), PactError>>, PactError> {
  let mut pacts = vec![];
  debug!("Scanning {:?}", dir);
  for entry in fs::read_dir(dir)? {
    let path = entry?.path();
    if path.is_dir() {
      pacts.extend(walkdir(&path, ext, s)?);
    } else if path.extension().is_some() && path.extension().unwrap_or_default() == ext {
      debug!("Loading file '{:?}'", path);
      pacts.push(read_pact(&path)
        .map(|p| (p, s.clone()))
        .map_err(|err| PactError::from(err).with_path(path.as_path())))
    }
  }
  Ok(pacts)
}

async fn pact_from_url(
  url: &str,
  auth: &Option<HttpAuth>,
  insecure_tls: bool,
  retries: u8,
) -> Result<Box<dyn Pact + Send + Sync + RefUnwindSafe>, PactError> {
  let client = if insecure_tls {
    warn!("Disabling TLS certificate validation");
    reqwest::Client::builder()
      .danger_accept_invalid_certs(true)
      .build()?
  } else {
    reqwest::Client::builder().build()?
  };
  let mut req = client.get(url);
  if let Some(u) = auth {
    req = match u {
      HttpAuth::User(user, password) => if let Some(pass) = password {
        req.header("Authorization", format!("Basic {}", Base64.encode(format!("{}:{}", user, pass))))
      } else {
        req.header("Authorization", format!("Basic {}", Base64.encode(user)))
      },
      HttpAuth::Token(token) => req.header("Authorization", format!("Bearer {}", token)),
      _ => req.header("Authorization", "undefined"),
    };
  }
  debug!("Executing Request to fetch pact from URL: {}", url);
  let pact_json: Value = with_retries(retries, req).await?.json().await?;
  debug!("Fetched Pact: {}", pact_json);
  load_pact_from_json(url, &pact_json).map_err(|err| err.into())
}

/// Load all the pact files from the provided sources
pub async fn load_pacts(
  sources: Vec<PactSource>,
  insecure_tls: bool,
  ext: Option<&String>,
  retries: u8,
) -> Vec<Result<(Box<dyn Pact + Send + Sync + RefUnwindSafe>, PactSource), PactError>> {
  futures::stream::iter(sources)
    .then(| s| async move {
      let values = match &s {
        PactSource::File(file) => vec![
          read_pact(Path::new(file))
            .map(|p| (p, s.clone()))
            .map_err(PactError::from)
        ],
        PactSource::Dir(dir) => match walkdir(Path::new(dir), ext.unwrap_or(&"json".to_string()), &s) {
          Ok(pacts) => pacts,
          Err(err) => vec![Err(PactError::new(format!("Could not load pacts from directory '{}' - {}", dir, err)))]
        },
        PactSource::URL(url, auth) => vec![
          pact_from_url(url, auth, insecure_tls, retries).await.map(|p| (p, s.clone()))
        ],
        PactSource::Broker { url, auth, consumers, providers } => {
          let mut http_builder = reqwest::ClientBuilder::new()
            .user_agent(format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")));
          if insecure_tls {
            http_builder = http_builder.danger_accept_invalid_certs(true);
          }
          let http_client = http_builder.build().unwrap();
          let client = HALClientBuilder::builder()
            .with_url(url, auth.clone())
            .with_http_client(http_client)
            .with_retries(retries)
            .build();
          match client.navigate("pb:latest-pact-versions", &hashmap!{}).await {
            Ok(client) => {
              match client.clone().iter_links("pb:pacts") {
                Ok(links) => {
                  futures::stream::iter(links.iter()
                    .map(|link| (link.clone(), client.clone())))
                    .then(|(link, client)| {
                      async move {
                        client.clone().fetch_url(&link, &hashmap!{}).await
                          .map_err(|err| PactError::new(err.to_string()))
                          .and_then(|json| {
                            let pact_title = link.title.clone().unwrap_or_else(|| link.href.clone().unwrap_or_default());
                            debug!("Found pact {}", pact_title);
                            load_pact_from_json(link.href.clone().unwrap_or_default().as_str(), &json)
                              .map_err(|err|
                                PactError::new(format!("Error loading \"{}\" ({}) - {}", pact_title, link.href.unwrap_or_default(), err))
                              )
                          })
                      }
                    })
                    .filter(|result| filter_consumers(consumers, result))
                    .filter(|result| filter_providers(providers, result))
                    .map(|result| result.map(|p| (p, s.clone())))
                    .collect().await
                },
                Err(err) => vec![Err(PactError::new(err.to_string()))]
              }
            }
            Err(err) => vec![Err(PactError::new(err.to_string()))]
          }
        }
        PactSource::Unknown => vec![]
      };
      futures::stream::iter(values)
    })
    .flatten()
    .collect()
    .await
}

fn filter_providers(providers: &Vec<Regex>, result: &Result<Box<dyn Pact + Send + Sync + RefUnwindSafe>, PactError>) -> Ready<bool> {
  match result {
    Ok(pact) => {
      if providers.is_empty() {
        ready(true)
      } else {
        let pact_name = pact.provider().name;
        ready(providers.iter().any(|name| name.is_match(&pact_name)))
      }
    }
    Err(_) => ready(true)
  }
}

fn filter_consumers(consumers: &Vec<Regex>, result: &Result<Box<dyn Pact + Send + Sync + RefUnwindSafe>, PactError>) -> Ready<bool> {
  match result {
    Ok(pact) => {
      if consumers.is_empty() {
        ready(true)
      } else {
        let pact_name = pact.consumer().name;
        ready(consumers.iter().any(|name| name.is_match(&pact_name)))
      }
    }
    Err(_) => ready(true)
  }
}

#[cfg(test)]
mod tests {
  use std::sync::atomic::{AtomicU32, Ordering};
  use std::sync::Arc;
  use std::time::Duration;

  use expectest::prelude::*;
  use pact_models::prelude::{Pact, RequestResponsePact};
  use regex::Regex;

  use crate::loading::{compute_retry_delay, filter_consumers, filter_providers, is_retryable, with_retries, PactError};

  // MARK: is_retryable unit tests

  #[test]
  fn is_retryable_returns_true_for_500() {
    assert!(is_retryable(reqwest::StatusCode::INTERNAL_SERVER_ERROR));
  }

  #[test]
  fn is_retryable_returns_true_for_429() {
    assert!(is_retryable(reqwest::StatusCode::TOO_MANY_REQUESTS));
  }

  #[test]
  fn is_retryable_returns_true_for_408() {
    assert!(is_retryable(reqwest::StatusCode::REQUEST_TIMEOUT));
  }

  #[test]
  fn is_retryable_returns_false_for_404() {
    assert!(!is_retryable(reqwest::StatusCode::NOT_FOUND));
  }

  #[test]
  fn is_retryable_returns_false_for_200() {
    assert!(!is_retryable(reqwest::StatusCode::OK));
  }

  // MARK: compute_retry_delay tests

  #[test]
  fn retry_delay_for_429_without_retry_after_uses_exponential_backoff() {
    // attempt=1: 500 * 2^0 = 500 ms
    assert_eq!(
      compute_retry_delay(reqwest::StatusCode::TOO_MANY_REQUESTS, None, 1),
      Duration::from_millis(500)
    );
    // attempt=2: 500 * 2^1 = 1000 ms
    assert_eq!(
      compute_retry_delay(reqwest::StatusCode::TOO_MANY_REQUESTS, None, 2),
      Duration::from_millis(1000)
    );
    // attempt=3: 500 * 2^2 = 2000 ms
    assert_eq!(
      compute_retry_delay(reqwest::StatusCode::TOO_MANY_REQUESTS, None, 3),
      Duration::from_millis(2000)
    );
  }

  #[test]
  fn retry_delay_for_429_with_retry_after_10_adds_20_percent() {
    assert_eq!(
      compute_retry_delay(reqwest::StatusCode::TOO_MANY_REQUESTS, Some(Duration::from_secs(10)), 1),
      Duration::from_secs(12)
    );
  }

  #[test]
  fn retry_delay_for_429_with_retry_after_400_caps_extra_at_60_seconds() {
    assert_eq!(
      compute_retry_delay(reqwest::StatusCode::TOO_MANY_REQUESTS, Some(Duration::from_secs(400)), 1),
      Duration::from_secs(460)
    );
  }

  // MARK: with_retries integration tests

  #[tokio::test]
  async fn with_retries_retries_429_responses() {
    use tokio::net::TcpListener;
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper::{Request, Response};
    use http_body_util::Full;
    use bytes::Bytes;

    let request_count = Arc::new(AtomicU32::new(0));
    let count_clone = request_count.clone();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
      loop {
        let (stream, _) = listener.accept().await.unwrap();
        let count = count_clone.clone();
        let io = hyper_util::rt::TokioIo::new(stream);
        tokio::spawn(async move {
          http1::Builder::new()
            .serve_connection(io, service_fn(move |_req: Request<hyper::body::Incoming>| {
              let n = count.fetch_add(1, Ordering::SeqCst);
              async move {
                if n < 2 {
                  Ok::<_, std::convert::Infallible>(
                    Response::builder()
                      .status(429)
                      .body(Full::new(Bytes::from("rate limited")))
                      .unwrap()
                  )
                } else {
                  Ok::<_, std::convert::Infallible>(
                    Response::builder()
                      .status(200)
                      .body(Full::new(Bytes::from("{}")))
                      .unwrap()
                  )
                }
              }
            }))
            .await
            .unwrap();
        });
      }
    });

    let client = reqwest::Client::new();
    let req = client.get(format!("http://{}/", addr));
    let result = with_retries(3, req).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().status(), 200);
    assert_eq!(request_count.load(Ordering::SeqCst), 3);
  }

  #[tokio::test]
  async fn with_retries_does_not_retry_404_responses() {
    use tokio::net::TcpListener;
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper::{Request, Response};
    use http_body_util::Full;
    use bytes::Bytes;

    let request_count = Arc::new(AtomicU32::new(0));
    let count_clone = request_count.clone();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
      loop {
        let (stream, _) = listener.accept().await.unwrap();
        let count = count_clone.clone();
        let io = hyper_util::rt::TokioIo::new(stream);
        tokio::spawn(async move {
          http1::Builder::new()
            .serve_connection(io, service_fn(move |_req: Request<hyper::body::Incoming>| {
              count.fetch_add(1, Ordering::SeqCst);
              async move {
                Ok::<_, std::convert::Infallible>(
                  Response::builder()
                    .status(404)
                    .body(Full::new(Bytes::from("not found")))
                    .unwrap()
                )
              }
            }))
            .await
            .unwrap();
        });
      }
    });

    let client = reqwest::Client::new();
    let req = client.get(format!("http://{}/", addr));
    let result = with_retries(3, req).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().status(), 404);
    assert_eq!(request_count.load(Ordering::SeqCst), 1);
  }

  #[tokio::test]
  async fn with_retries_zero_sends_exactly_once_without_panicking() {
    use tokio::net::TcpListener;
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper::{Request, Response};
    use http_body_util::Full;
    use bytes::Bytes;

    let request_count = Arc::new(AtomicU32::new(0));
    let count_clone = request_count.clone();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
      loop {
        let (stream, _) = listener.accept().await.unwrap();
        let count = count_clone.clone();
        let io = hyper_util::rt::TokioIo::new(stream);
        tokio::spawn(async move {
          http1::Builder::new()
            .serve_connection(io, service_fn(move |_req: Request<hyper::body::Incoming>| {
              count.fetch_add(1, Ordering::SeqCst);
              async move {
                Ok::<_, std::convert::Infallible>(
                  Response::builder()
                    .status(429)
                    .body(Full::new(Bytes::from("rate limited")))
                    .unwrap()
                )
              }
            }))
            .await
            .unwrap();
        });
      }
    });

    let client = reqwest::Client::new();
    let req = client.get(format!("http://{}/", addr));
    let result = with_retries(0, req).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().status(), 429);
    assert_eq!(request_count.load(Ordering::SeqCst), 1);
  }

  #[tokio::test]
  async fn with_retries_handles_http_date_retry_after_header() {
    use tokio::net::TcpListener;
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper::{Request, Response};
    use http_body_util::Full;
    use bytes::Bytes;

    let request_count = Arc::new(AtomicU32::new(0));
    let count_clone = request_count.clone();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
      loop {
        let (stream, _) = listener.accept().await.unwrap();
        let count = count_clone.clone();
        let io = hyper_util::rt::TokioIo::new(stream);
        tokio::spawn(async move {
          http1::Builder::new()
            .serve_connection(io, service_fn(move |_req: Request<hyper::body::Incoming>| {
              let n = count.fetch_add(1, Ordering::SeqCst);
              async move {
                if n < 1 {
                  Ok::<_, std::convert::Infallible>(
                    Response::builder()
                      .status(429)
                      // HTTP-date in the past → duration_since returns zero → no delay
                      .header("Retry-After", "Thu, 01 Jan 1970 00:00:00 GMT")
                      .body(Full::new(Bytes::from("rate limited")))
                      .unwrap()
                  )
                } else {
                  Ok::<_, std::convert::Infallible>(
                    Response::builder()
                      .status(200)
                      .body(Full::new(Bytes::from("{}")))
                      .unwrap()
                  )
                }
              }
            }))
            .await
            .unwrap();
        });
      }
    });

    let client = reqwest::Client::new();
    let req = client.get(format!("http://{}/", addr));
    let result = with_retries(3, req).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().status(), 200);
    assert_eq!(request_count.load(Ordering::SeqCst), 2);
  }

  #[tokio::test]
  async fn with_retries_retries_408_responses() {
    use tokio::net::TcpListener;
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper::{Request, Response};
    use http_body_util::Full;
    use bytes::Bytes;

    let request_count = Arc::new(AtomicU32::new(0));
    let count_clone = request_count.clone();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
      loop {
        let (stream, _) = listener.accept().await.unwrap();
        let count = count_clone.clone();
        let io = hyper_util::rt::TokioIo::new(stream);
        tokio::spawn(async move {
          http1::Builder::new()
            .serve_connection(io, service_fn(move |_req: Request<hyper::body::Incoming>| {
              let n = count.fetch_add(1, Ordering::SeqCst);
              async move {
                if n == 0 {
                  Ok::<_, std::convert::Infallible>(
                    Response::builder()
                      .status(408)
                      .body(Full::new(Bytes::from("request timeout")))
                      .unwrap()
                  )
                } else {
                  Ok::<_, std::convert::Infallible>(
                    Response::builder()
                      .status(200)
                      .body(Full::new(Bytes::from("{}")))
                      .unwrap()
                  )
                }
              }
            }))
            .await
            .unwrap();
        });
      }
    });

    let client = reqwest::Client::new();
    let req = client.get(format!("http://{}/", addr));
    let result = with_retries(3, req).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().status(), 200);
    assert_eq!(request_count.load(Ordering::SeqCst), 2);
  }

  #[tokio::test]
  async fn filter_consumers_with_error_result() {
    let result = Err(PactError::new("test".to_string()));
    let filter_result = filter_consumers(&vec![Regex::new("one").unwrap()], &result).await;
    expect!(filter_result).to(be_true());
  }

  #[tokio::test]
  async fn filter_consumers_with_no_consumers() {
    let result = Ok(RequestResponsePact::default().boxed());
    let filter_result = filter_consumers(&vec![], &result).await;
    expect!(filter_result).to(be_true());
  }

  #[tokio::test]
  async fn filter_consumers_with_no_matching_consumer_name() {
    let result = Ok(RequestResponsePact::default().boxed());
    let names = vec![
      Regex::new("one").unwrap(),
      Regex::new("two").unwrap(),
      Regex::new("three").unwrap()
    ];
    let filter_result = filter_consumers(&names, &result).await;
    expect!(filter_result).to(be_false());
  }

  #[tokio::test]
  async fn filter_consumers_with_a_matching_consumer_name() {
    let result = Ok(RequestResponsePact::default().boxed());
    let names = vec![
      Regex::new("one").unwrap(),
      Regex::new("two").unwrap(),
      Regex::new("default_consumer").unwrap()
    ];
    let filter_result = filter_consumers(&names, &result).await;
    expect!(filter_result).to(be_true());
  }

  #[tokio::test]
  async fn filter_consumers_with_a_matching_consumer_name_with_regex() {
    let result = Ok(RequestResponsePact::default().boxed());
    let names = vec![
      Regex::new("one").unwrap(),
      Regex::new("two").unwrap(),
      Regex::new("\\w+_consumer").unwrap()
    ];
    let filter_result = filter_consumers(&names, &result).await;
    expect!(filter_result).to(be_true());
  }

  #[tokio::test]
  async fn filter_providers_with_error_result() {
    let result = Err(PactError::new("test".to_string()));
    let filter_result = filter_providers(&vec![Regex::new("one").unwrap()], &result).await;
    expect!(filter_result).to(be_true());
  }

  #[tokio::test]
  async fn filter_providers_with_no_providers() {
    let result = Ok(RequestResponsePact::default().boxed());
    let filter_result = filter_providers(&vec![], &result).await;
    expect!(filter_result).to(be_true());
  }

  #[tokio::test]
  async fn filter_providers_with_no_matching_provider_name() {
    let result = Ok(RequestResponsePact::default().boxed());
    let names = vec![
      Regex::new("one").unwrap(),
      Regex::new("two").unwrap(),
      Regex::new("three").unwrap()
    ];
    let filter_result = filter_providers(&names, &result).await;
    expect!(filter_result).to(be_false());
  }

  #[tokio::test]
  async fn filter_providers_with_a_matching_provider_name() {
    let result = Ok(RequestResponsePact::default().boxed());
    let names = vec![
      Regex::new("one").unwrap(),
      Regex::new("two").unwrap(),
      Regex::new("default_provider").unwrap()
    ];
    let filter_result = filter_providers(&names, &result).await;
    expect!(filter_result).to(be_true());
  }

  #[tokio::test]
  async fn filter_providers_with_a_matching_provider_name_with_regex() {
    let result = Ok(RequestResponsePact::default().boxed());
    let names = vec![
      Regex::new("one").unwrap(),
      Regex::new("two").unwrap(),
      Regex::new("\\w+_provider").unwrap()
    ];
    let filter_result = filter_providers(&names, &result).await;
    expect!(filter_result).to(be_true());
  }
}
