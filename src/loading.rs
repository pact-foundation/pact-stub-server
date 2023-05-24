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
use pact_verifier::pact_broker::HALClient;
use regex::Regex;
use serde_json::Value;
use tracing::{debug, warn};

use crate::PactSource;

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
  insecure_tls: bool
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
  let pact_json: Value = req.send().await?.json().await?;
  debug!("Fetched Pact: {}", pact_json);
  load_pact_from_json(url, &pact_json).map_err(|err| err.into())
}

/// Load all the pact files from the provided sources
pub async fn load_pacts(
  sources: Vec<PactSource>,
  insecure_tls: bool,
  ext: Option<&String>
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
          pact_from_url(url, auth, insecure_tls).await.map(|p| (p, s.clone()))
        ],
        PactSource::Broker { url, auth, consumers, providers } => {
          let client = HALClient::with_url(url, auth.clone());
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
  use expectest::prelude::*;
  use pact_models::prelude::{Pact, RequestResponsePact};
  use regex::Regex;

  use crate::loading::{filter_consumers, filter_providers, PactError};

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
