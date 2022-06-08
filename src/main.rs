//! # Standalone Pact Stub Server
//!
//! This project provides a server that can generate responses based on pact files. It is a single executable binary. It implements the [V2 Pact specification](https://github.com/pact-foundation/pact-specification/tree/version-2).
//!
//! [Online rust docs](https://docs.rs/pact-stub-server/)
//!
//! The stub server works by taking all the interactions (requests and responses) from a number of pact files. For each interaction, it will compare any incoming request against those defined in the pact files. If there is a match (based on method, path and query parameters), it will return the response from the pact file.
//!
//! ## Command line interface
//!
//! The pact stub server is bundled as a single binary executable `pact-stub-server`. Running this with out any options displays the standard help.
//!
//! ```console,ignore
//! pact-stub-server v0.0.2
//! Pact Stub Server
//!
//! USAGE:
//!     pact-stub-server [OPTIONS] --file <file> --dir <dir> --url <url>
//!
//! FLAGS:
//!     -h, --help       Prints help information
//!     -v, --version    Prints version information
//!
//! OPTIONS:
//!     -d, --dir <dir>              Directory of pact files to verify (can be repeated)
//!     -f, --file <file>            Pact file to verify (can be repeated)
//!     -l, --loglevel <loglevel>    Log level (defaults to info) [values: error, warn, info, debug, trace, none]
//!     -p, --port <port>            Port to run on (defaults to random port assigned by the OS)
//!     -s, --provider-state <provider-state>    Provider state regular expression to filter the responses by
//!         --provider-state-header-name <name>  Name of the header parameter containing the
//! provider state to be used in case multiple matching interactions are found
//!     -u, --url <url>              URL of pact file to verify (can be repeated)
//!
//! ```
//!
//! ## Options
//!
//! ### Log Level
//!
//! You can control the log level with the `-l, --loglevel <loglevel>` option. It defaults to info, and the options that you can specify are: error, warn, info, debug, trace, none.
//!
//! ### Pact File Sources
//!
//! You can specify the pacts to verify with the following options. They can be repeated to set multiple sources.
//!
//! | Option | Type | Description |
//! |--------|------|-------------|
//! | `-f, --file <file>` | File | Loads a pact from the given file |
//! | `-u, --url <url>` | URL | Loads a pact from a URL resource |
//! | `-d, --dir <dir>` | Directory | Loads all the pacts from the given directory |
//!
//! ### Server Options
//!
//! The running server can be controlled with the following options:
//!
//! | Option | Description |
//! |--------|-------------|
//! | `-p, --port <port>` | The port to bind to. If not specified, a random port will be allocated by the operating system. |
//!

#![warn(missing_docs)]

use std::env;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::Path;
use std::process::ExitCode;
use std::str::FromStr;

use base64::encode;
use clap::{Command, Arg, ArgMatches, ErrorKind};
use futures::stream::*;
use maplit::*;
use pact_models::pact::{load_pact_from_json, read_pact};
use pact_models::prelude::*;
use pact_verifier::pact_broker::HALClient;
use regex::Regex;
use serde_json::Value;
use tracing::{debug, error, warn};
use tracing_core::LevelFilter;
use tracing_subscriber::FmtSubscriber;

use crate::server::ServerHandler;

mod pact_support;
mod server;

#[derive(Debug, Clone)]
struct PactError {
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

#[tokio::main]
async fn main() -> Result<(), ExitCode> {
  let args: Vec<String> = env::args().collect();
  match handle_command_args(args).await {
    Ok(_) => Ok(()),
    Err(err) => Err(ExitCode::from(err))
  }
}

fn print_version() {
    println!("pact stub server version  : v{}", env!("CARGO_PKG_VERSION"));
    println!("pact specification version: v{}", PactSpecification::V4.version_str());
}

fn integer_value(v: &str) -> Result<(), String> {
    v.parse::<u16>().map(|_| ()).map_err(|e| format!("'{}' is not a valid port value: {}", v, e) )
}

fn regex_value(v: &str) -> Result<(), String> {
    Regex::new(v).map(|_| ()).map_err(|e| format!("'{}' is not a valid regular expression: {}", v, e) )
}

/// Source for loading pacts
#[derive(Debug, Clone)]
pub enum PactSource {
  /// Load the pact from a pact file
  File(String),
  /// Load all the pacts from a Directory
  Dir(String),
  /// Load the pact from a URL
  URL(String, Option<HttpAuth>),
  /// Load all pacts from a Pact Broker
  Broker(String, Option<HttpAuth>)
}

fn pact_source(matches: &ArgMatches) -> Vec<PactSource> {
  let mut sources = vec![];
  if let Some(values) = matches.values_of("file") {
    sources.extend(values.map(|v| PactSource::File(v.to_string())).collect::<Vec<PactSource>>());
  }
  if let Some(values) = matches.values_of("dir") {
    sources.extend(values.map(|v| PactSource::Dir(v.to_string())).collect::<Vec<PactSource>>());
  }
  if let Some(values) = matches.values_of("url") {
    sources.extend(values.map(|v| {
      let auth = matches.value_of("user").map(|u| {
        let mut auth = u.split(':');
        HttpAuth::User(auth.next().unwrap().to_string(), auth.next().map(|p| p.to_string()))
      })
        .or_else(|| matches.value_of("token").map(|v| HttpAuth::Token(v.to_string())));
      PactSource::URL(v.to_string(), auth)
    }).collect::<Vec<PactSource>>());
  }
  if let Some(url) = matches.value_of("broker-url") {
    let auth = matches.value_of("user").map(|u| {
      let mut auth = u.split(':');
      HttpAuth::User(auth.next().unwrap().to_string(), auth.next().map(|p| p.to_string()))
    }).or_else(|| matches.value_of("token").map(|v| HttpAuth::Token(v.to_string())));
    debug!("Loading all pacts from Pact Broker at {} using {} authentication", url,
      auth.clone().map(|auth| auth.to_string()).unwrap_or_else(|| "no".to_string()));
    sources.push(PactSource::Broker(url.to_string(), auth));
  }
  sources
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

async fn load_pacts(
  sources: Vec<PactSource>,
  insecure_tls: bool,
  ext: Option<&str>
) -> Vec<Result<Box<dyn Pact + Send + Sync>, PactError>> {
  futures::stream::iter(sources.iter().cloned()).then(| s| async move {
    let val = match &s {
      PactSource::File(file) => vec![
        read_pact(Path::new(file)).map_err(PactError::from)
      ],
      PactSource::Dir(dir) => match walkdir(Path::new(dir), ext.unwrap_or("json")) {
        Ok(pacts) => pacts,
        Err(err) => vec![Err(PactError::new(format!("Could not load pacts from directory '{}' - {}", dir, err)))]
      },
      PactSource::URL(url, auth) => vec![pact_from_url(url, auth, insecure_tls).await],
      PactSource::Broker(url, auth) => {
        let client = HALClient::with_url(url, auth.clone());
        match client.navigate("pb:latest-pact-versions", &hashmap!{}).await {
          Ok(client) => {
            match client.clone().iter_links("pb:pacts") {
              Ok(links) => {
                futures::stream::iter(links.iter().map(|link| (link.clone(), client.clone())))
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
                  .collect().await
              },
              Err(err) => vec![Err(PactError::new(err.to_string()))]
            }
          }
          Err(err) => vec![Err(PactError::new(err.to_string()))]
        }
      }
    };
    futures::stream::iter(val)
  }).flatten().collect().await
}

async fn handle_command_args(args: Vec<String>) -> Result<(), u8> {
  let program = args[0].clone();
  let version = format!("v{}", env!("CARGO_PKG_VERSION"));
  let app = build_args(program.as_str(), version.as_str());
  match app.try_get_matches_from(args) {
    Ok(ref matches) => {
      let level = matches.value_of("loglevel").unwrap_or("info");
      setup_logger(level);
      let sources = pact_source(matches);

      let pacts = load_pacts(sources, matches.is_present("insecure-tls"),
        matches.value_of("ext")).await;
      if pacts.iter().any(|p| p.is_err()) {
        error!("There were errors loading the pact files.");
        for error in pacts.iter()
          .filter(|p| p.is_err())
          .map(|e| match e {
            Err(err) => err.clone(),
            _ => panic!("Internal Code Error - was expecting an error but was not")
          }) {
          error!("  - {}", error);
        }
        Err(3)
      } else {
        let port = matches.value_of("port").unwrap_or("0").parse::<u16>().unwrap();
        let provider_state = matches.value_of("provider-state")
            .map(|filter| Regex::new(filter).unwrap());
        let provider_state_header_name = matches.value_of("provider-state-header-name")
            .map(String::from);
        let empty_provider_states = matches.is_present("empty-provider-state");
        let pacts = pacts.iter()
          .map(|result| {
            // Currently, as_v4_pact won't fail as it upgrades older formats to V4, so is safe to unwrap
            result.as_ref().unwrap().as_v4_pact().unwrap()
          })
          .collect();
        let auto_cors = matches.is_present("cors");
        let referer = matches.is_present("cors-referer");
        let server_handler = ServerHandler::new(
          pacts,
          auto_cors,
          referer,
          provider_state,
          provider_state_header_name,
          empty_provider_states);
        tokio::task::spawn_blocking(move || {
          server_handler.start_server(port)
        }).await.unwrap()
      }
    },
    Err(ref err) => {
      match err.kind() {
        ErrorKind::DisplayHelp => {
          println!("{}", err);
          Ok(())
        },
        ErrorKind::DisplayVersion => {
          print_version();
          println!();
          Ok(())
        },
        _ => err.exit()
      }
    }
  }
}

fn build_args<'a>(program: &'a str, version: &'a str) -> Command<'a> {
  Command::new(program)
    .version(version)
    .about("Pact Stub Server")
    .arg_required_else_help(true)
    .mut_arg("version", |a| a.short('v'))
    .arg(Arg::new("loglevel")
      .short('l')
      .long("loglevel")
      .takes_value(true)
      .use_value_delimiter(false)
      .possible_values(&["error", "warn", "info", "debug", "trace", "none"])
      .help("Log level (defaults to info)"))
    .arg(Arg::new("file")
      .short('f')
      .long("file")
      .required_unless_present_any(&["dir", "url", "broker-url"])
      .takes_value(true)
      .use_value_delimiter(false)
      .multiple_occurrences(true)
      .number_of_values(1)
      .forbid_empty_values(true)
      .help("Pact file to load (can be repeated)"))
    .arg(Arg::new("dir")
      .short('d')
      .long("dir")
      .required_unless_present_any(&["file", "url", "broker-url"])
      .takes_value(true)
      .use_value_delimiter(false)
      .multiple_occurrences(true)
      .number_of_values(1)
      .forbid_empty_values(true)
      .help("Directory of pact files to load (can be repeated)"))
    .arg(Arg::new("ext")
      .short('e')
      .long("extension")
      .takes_value(true)
      .use_value_delimiter(false)
      .number_of_values(1)
      .forbid_empty_values(true)
      .requires("dir")
      .help("File extension to use when loading from a directory (default is json)"))
    .arg(Arg::new("url")
      .short('u')
      .long("url")
      .required_unless_present_any(&["file", "dir", "broker-url"])
      .takes_value(true)
      .use_value_delimiter(false)
      .multiple_occurrences(true)
      .number_of_values(1)
      .forbid_empty_values(true)
      .help("URL of pact file to fetch (can be repeated)"))
    .arg(Arg::new("broker-url")
      .short('b')
      .long("broker-url")
      .env("PACT_BROKER_BASE_URL")
      .required_unless_present_any(&["file", "dir", "url"])
      .takes_value(true)
      .use_value_delimiter(false)
      .multiple_occurrences(false)
      .number_of_values(1)
      .forbid_empty_values(true)
      .help("URL of the pact broker to fetch pacts from"))
    .arg(Arg::new("user")
      .long("user")
      .takes_value(true)
      .use_value_delimiter(false)
      .number_of_values(1)
      .forbid_empty_values(true)
      .conflicts_with("token")
      .help("User and password to use when fetching pacts from URLS or Pact Broker in user:password form"))
    .arg(Arg::new("token")
      .short('t')
      .long("token")
      .takes_value(true)
      .use_value_delimiter(false)
      .number_of_values(1)
      .forbid_empty_values(true)
      .conflicts_with("user")
      .help("Bearer token to use when fetching pacts from URLS or Pact Broker"))
    .arg(Arg::new("port")
      .short('p')
      .long("port")
      .takes_value(true)
      .use_value_delimiter(false)
      .help("Port to run on (defaults to random port assigned by the OS)")
      .validator(integer_value))
    .arg(Arg::new("cors")
      .short('o')
      .long("cors")
      .help("Automatically respond to OPTIONS requests and return default CORS headers"))
    .arg(Arg::new("cors-referer")
      .long("cors-referer")
      .requires("cors")
      .help("Set the CORS Access-Control-Allow-Origin header to the Referer"))
    .arg(Arg::new("insecure-tls")
      .long("insecure-tls")
      .help("Disables TLS certificate validation"))
    .arg(Arg::new("provider-state")
      .short('s')
      .long("provider-state")
      .takes_value(true)
      .use_value_delimiter(false)
      .number_of_values(1)
      .forbid_empty_values(true)
      .validator(regex_value)
      .help("Provider state regular expression to filter the responses by"))
    .arg(Arg::new("provider-state-header-name")
      .long("provider-state-header-name")
      .takes_value(true)
      .use_value_delimiter(false)
      .number_of_values(1)
      .forbid_empty_values(true)
      .help("Name of the header parameter containing the provider state to be used in case \
      multiple matching interactions are found"))
    .arg(Arg::new("empty-provider-state")
      .long("empty-provider-state")
      .requires("provider-state")
      .help("Include empty provider states when filtering with --provider-state"))
}

fn setup_logger(level: &str) {
  let log_level = match level {
    "none" => LevelFilter::OFF,
    _ => LevelFilter::from_str(level).unwrap_or(LevelFilter::INFO)
  };
  let subscriber = FmtSubscriber::builder()
    .with_max_level(log_level)
    .with_thread_names(true)
    .finish();
  if let Err(err) = tracing::subscriber::set_global_default(subscriber) {
    eprintln!("ERROR: Failed to initialise global tracing subscriber - {err}");
  };
}

#[cfg(test)]
mod test;
