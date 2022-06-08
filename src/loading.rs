//! Functions relating to loading Pact files

use std::fmt::{Display, Formatter};
use std::fs;
use std::path::Path;

use base64::encode;
use futures::future::ready;
use futures::StreamExt;
use maplit::hashmap;
use pact_models::pact::{load_pact_from_json, read_pact};
use pact_models::prelude::*;
use pact_verifier::pact_broker::HALClient;
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

fn walkdir(dir: &Path, ext: &str) -> Result<Vec<Result<Box<dyn Pact + Send + Sync>, PactError>>, PactError> {
  let mut pacts = vec![];
  debug!("Scanning {:?}", dir);
  for entry in fs::read_dir(dir)? {
    let path = entry?.path();
    if path.is_dir() {
      walkdir(&path, ext)?;
    } else if path.extension().is_some() && path.extension().unwrap_or_default() == ext {
      debug!("Loading file '{:?}'", path);
      pacts.push(read_pact(&path)
        .map_err(|err| PactError::from(err).with_path(path.as_path())))
    }
  }
  Ok(pacts)
}

async fn pact_from_url(
  url: &str,
  auth: &Option<HttpAuth>,
  insecure_tls: bool
) -> Result<Box<dyn Pact + Send + Sync>, PactError> {
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
        req.header("Authorization", format!("Basic {}", encode(format!("{}:{}", user, pass))))
      } else {
        req.header("Authorization", format!("Basic {}", encode(user)))
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
  ext: Option<&str>
) -> Vec<Result<Box<dyn Pact + Send + Sync>, PactError>> {
  futures::stream::iter(sources)
    .then(| s| async move {
      let values = match &s {
        PactSource::File(file) => vec![
          read_pact(Path::new(file)).map_err(PactError::from)
        ],
        PactSource::Dir(dir) => match walkdir(Path::new(dir), ext.unwrap_or("json")) {
          Ok(pacts) => pacts,
          Err(err) => vec![Err(PactError::new(format!("Could not load pacts from directory '{}' - {}", dir, err)))]
        },
        PactSource::URL(url, auth) => vec![ pact_from_url(url, auth, insecure_tls).await ],
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
                    .filter(|result| {
                      match result {
                        Ok(pact) => {
                          if consumers.is_empty() {
                            ready(true)
                          } else {
                            ready(consumers.contains(&pact.consumer().name))
                          }
                        }
                        Err(_) => ready(true)
                      }
                    })
                    .filter(|result| {
                      match result {
                        Ok(pact) => {
                          if providers.is_empty() {
                            ready(true)
                          } else {
                            ready(providers.contains(&pact.provider().name))
                          }
                        }
                        Err(_) => ready(true)
                      }
                    })
                    .collect().await
                },
                Err(err) => vec![Err(PactError::new(err.to_string()))]
              }
            }
            Err(err) => vec![Err(PactError::new(err.to_string()))]
          }
        }
      };
      futures::stream::iter(values)
    })
    .flatten()
    .collect()
    .await
}
