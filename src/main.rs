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

use clap::{App, AppSettings, Arg, ArgMatches, ErrorKind};
use log::LevelFilter;
use pact_matching::models::{Pact, PactSpecification};
use simplelog::{Config, SimpleLogger, TermLogger, TerminalMode};
use std::env;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use base64::encode;
use regex::Regex;
use std::error::Error;
use std::fmt::Display;
use serde::export::Formatter;
use futures::stream::*;
use crate::server::ServerHandler;
use pact_matching::s;
use log::*;
use clap::crate_version;

mod pact_support;
mod server;

#[derive(Debug, Clone)]
struct PactError {
  message: String
}

impl PactError {
  fn new(str: String) -> PactError {
    PactError { message: str }
  }
}

impl Display for PactError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.message)
  }
}

impl From<reqwest::Error> for PactError {
  fn from(err: reqwest::Error) -> Self {
    PactError { message: format!("Request failed: {}", err) }
  }
}

impl From<serde_json::error::Error> for PactError {
  fn from(err: serde_json::error::Error) -> Self {
    PactError { message: format!("Failed to parse JSON body: {}", err) }
  }
}

impl From<std::io::Error> for PactError {
  fn from(err: std::io::Error) -> Self {
    PactError { message: format!("Failed to load pact file: {}", err) }
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  match handle_command_args().await {
    Ok(_) => Ok(()),
    Err(err) => std::process::exit(err)
  }
}

fn print_version() {
    println!("\npact stub server version  : v{}", crate_version!());
    println!("pact specification version: v{}", PactSpecification::V3.version_str());
}

fn integer_value(v: String) -> Result<(), String> {
    v.parse::<u16>().map(|_| ()).map_err(|e| format!("'{}' is not a valid port value: {}", v, e) )
}

fn regex_value(v: String) -> Result<(), String> {
    Regex::new(v.as_str()).map(|_| ()).map_err(|e| format!("'{}' is not a valid regular expression: {}", v, e) )
}

/// Type of authentication to use
#[derive(Debug, Clone)]
pub enum UrlAuth {
  /// Username and Password
  User(String),
  /// Bearer token
  Token(String)
}

/// Source for loading pacts
#[derive(Debug, Clone)]
pub enum PactSource {
    /// Load the pact from a pact file
    File(String),
    /// Load all the pacts from a Directory
    Dir(String),
    /// Load the pact from a URL
    URL(String, Option<UrlAuth>)
}

fn pact_source(matches: &ArgMatches) -> Vec<PactSource> {
    let mut sources = vec![];
    match matches.values_of("file") {
        Some(values) => sources.extend(values.map(|v| PactSource::File(s!(v))).collect::<Vec<PactSource>>()),
        None => ()
    };
    match matches.values_of("dir") {
        Some(values) => sources.extend(values.map(|v| PactSource::Dir(s!(v))).collect::<Vec<PactSource>>()),
        None => ()
    };
    match matches.values_of("url") {
        Some(values) => sources.extend(values.map(|v| {
          let auth = matches.value_of("user").map(|u| UrlAuth::User(u.to_string()))
            .or(matches.value_of("token").map(|v| UrlAuth::Token(v.to_string())));
          PactSource::URL(s!(v), auth)
        }).collect::<Vec<PactSource>>()),
        None => ()
    };
    sources
}

fn walkdir(dir: &Path) -> Result<Vec<Result<Pact, PactError>>, PactError> {
    let mut pacts = vec![];
    debug!("Scanning {:?}", dir);
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            walkdir(&path)?;
        } else {
            pacts.push(Pact::read_pact(&path).map_err(|err| PactError::from(err)))
        }
    }
    Ok(pacts)
}

async fn pact_from_url(url: &String, auth: &Option<UrlAuth>, insecure_tls: bool) -> Result<Pact, PactError> {
  let client = if insecure_tls {
    warn!("Disabling TLS certificate validation");
    reqwest::Client::builder()
      .danger_accept_invalid_hostnames(true)
      .danger_accept_invalid_certs(true)
      .build()?
  } else {
    reqwest::Client::builder().build()?
  };
  let mut req = client.get(url);
  if let Some(ref u) = auth {
    req = match u {
      &UrlAuth::User(ref user) => req.header("Authorization", format!("Basic {}", encode(&user))),
      &UrlAuth::Token(ref token) => req.header("Authorization", format!("Bearer {}", token))
    };
  }
  debug!("Executing Request to fetch pact from URL: {}", url);
  let res = req.send().await?.text().await?;
  let pact_json = serde_json::from_str(&res)?;
  let pact = Pact::from_json(&url, &pact_json);
  debug!("Fetched Pact: {:?}", pact);
  Ok(pact)
}

async fn load_pacts(sources: Vec<PactSource>, insecure_tls: bool) -> Vec<Result<Pact, PactError>> {
  futures::stream::iter(sources.iter().cloned()).then(| s| async move {
    let val = match s {
      PactSource::File(ref file) => vec![Pact::read_pact(Path::new(file)).map_err(|err| PactError::from(err))],
      PactSource::Dir(ref dir) => match walkdir(Path::new(dir)) {
        Ok(ref pacts) => pacts.iter().cloned().map(|res| res.map_err(|err| PactError::from(err))).collect(),
        Err(err) => vec![Err(PactError::new(format!("Could not load pacts from directory '{}' - {}", dir, err)))]
      },
      PactSource::URL(ref url, ref auth) => vec![pact_from_url(url, auth, insecure_tls).await]
    };
    futures::stream::iter(val.clone())
  }).flatten().collect().await
}

async fn handle_command_args() -> Result<(), i32> {
  let args: Vec<String> = env::args().collect();
  let program = args[0].clone();

  let version = format!("v{}", crate_version!());
  let app = App::new(program)
      .version(version.as_str())
      .about("Pact Stub Server")
      .version_short("v")
      .setting(AppSettings::ArgRequiredElseHelp)
      .setting(AppSettings::ColoredHelp)
      .arg(Arg::with_name("loglevel")
          .short("l")
          .long("loglevel")
          .takes_value(true)
          .use_delimiter(false)
          .possible_values(&["error", "warn", "info", "debug", "trace", "none"])
          .help("Log level (defaults to info)"))
      .arg(Arg::with_name("file")
          .short("f")
          .long("file")
          .required_unless_one(&["dir", "url"])
          .takes_value(true)
          .use_delimiter(false)
          .multiple(true)
          .number_of_values(1)
          .empty_values(false)
          .help("Pact file to verify (can be repeated)"))
      .arg(Arg::with_name("dir")
          .short("d")
          .long("dir")
          .required_unless_one(&["file", "url"])
          .takes_value(true)
          .use_delimiter(false)
          .multiple(true)
          .number_of_values(1)
          .empty_values(false)
          .help("Directory of pact files to verify (can be repeated)"))
      .arg(Arg::with_name("url")
          .short("u")
          .long("url")
          .required_unless_one(&["file", "dir"])
          .takes_value(true)
          .use_delimiter(false)
          .multiple(true)
          .number_of_values(1)
          .empty_values(false)
          .help("URL of pact file to verify (can be repeated)"))
      .arg(Arg::with_name("user")
        .long("user")
        .takes_value(true)
        .use_delimiter(false)
        .number_of_values(1)
        .empty_values(false)
        .conflicts_with("token")
        .help("User and password to use when fetching pacts from URLS in user:password form"))
      .arg(Arg::with_name("token")
        .short("t")
        .long("token")
        .takes_value(true)
        .use_delimiter(false)
        .number_of_values(1)
        .empty_values(false)
        .conflicts_with("user")
        .help("Bearer token to use when fetching pacts from URLS"))
      .arg(Arg::with_name("port")
          .short("p")
          .long("port")
          .takes_value(true)
          .use_delimiter(false)
          .help("Port to run on (defaults to random port assigned by the OS)")
          .validator(integer_value))
      .arg(Arg::with_name("cors")
          .short("o")
          .long("cors")
          .takes_value(false)
          .use_delimiter(false)
          .help("Automatically respond to OPTIONS requests and return default CORS headers"))
      .arg(Arg::with_name("cors-referer")
          .long("cors-referer")
          .takes_value(false)
          .use_delimiter(false)
          .requires("cors")
          .help("Set the CORS Access-Control-Allow-Origin header to the Referer"))
      .arg(Arg::with_name("insecure-tls")
          .long("insecure-tls")
          .takes_value(false)
          .use_delimiter(false)
          .help("Disables TLS certificate validation"))
      .arg(Arg::with_name("provider-state")
          .short("s")
          .long("provider-state")
          .takes_value(true)
          .use_delimiter(false)
          .number_of_values(1)
          .empty_values(false)
          .validator(regex_value)
          .help("Provider state regular expression to filter the responses by"))
      .arg(Arg::with_name("provider-state-header-name")
          .long("provider-state-header-name")
          .takes_value(true)
          .use_delimiter(false)
          .number_of_values(1)
          .empty_values(false)
          .help("Name of the header parameter containing the provider state to be used in case \
          multiple matching interactions are found"));

  let matches = app.get_matches_safe();
  match matches {
    Ok(ref matches) => {
      let level = matches.value_of("loglevel").unwrap_or("info");
      setup_logger(level);
      let sources = pact_source(matches);

      let pacts = load_pacts(sources, matches.is_present("insecure-tls")).await;
      debug!("pacts = {:?}", pacts);
      if pacts.iter().any(|p| p.is_err()) {
        error!("There were errors loading the pact files.");
        for error in pacts.iter().filter(|p| p.is_err()).cloned().map(|e| e.unwrap_err()) {
          error!("  - {}", error);
        }
        Err(3)
      } else {
        let port = matches.value_of("port").unwrap_or("0").parse::<u16>().unwrap();
        let provider_state = matches.value_of("provider-state")
            .map(|filter| Regex::new(filter).unwrap());
        let provider_state_header_name = matches.value_of("provider-state-header-name")
            .map(|filter| String::from(filter));
        let pacts = pacts.iter().cloned().map(|p| p.unwrap()).collect();
        let server_handler = ServerHandler::new(pacts, matches.is_present("cors"),
          matches.is_present("cors-referer"), provider_state, provider_state_header_name);
        server_handler.start_server(port)
      }
    },
    Err(ref err) => {
      match err.kind {
        ErrorKind::HelpDisplayed => {
          println!("{}", err.message);
          Ok(())
        },
        ErrorKind::VersionDisplayed => {
          print_version();
          println!();
          Ok(())
        },
        _ => err.exit()
      }
    }
  }
}

fn setup_logger(level: &str) {
    let log_level = match level {
        "none" => LevelFilter::Off,
        _ => LevelFilter::from_str(level).unwrap()
    };
    match TermLogger::init(log_level, Config::default(), TerminalMode::Mixed) {
        Err(_) => SimpleLogger::init(log_level, Config::default()).unwrap_or(()),
        Ok(_) => ()
    }
}

#[cfg(test)]
mod test;
