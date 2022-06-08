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
use std::process::ExitCode;
use std::str::FromStr;

use clap::{Command, Arg, ArgMatches, ErrorKind};
use itertools::Itertools;
use pact_models::prelude::*;
use regex::Regex;
use tracing::{debug, error, info, warn};
use tracing_core::LevelFilter;
use tracing_subscriber::FmtSubscriber;
use crate::loading::load_pacts;

use crate::server::ServerHandler;

mod pact_support;
mod server;
mod loading;

#[tokio::main]
async fn main() -> Result<(), ExitCode> {
  let args: Vec<String> = env::args().collect();
  handle_command_args(args).await
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
  Broker {
    /// Broker URL
    url: String,
    /// Any required auth
    auth: Option<HttpAuth>,
    /// Consumer names to filter Pacts with
    consumers: Vec<String>,
    /// Provider names to filter Pacts with
    providers: Vec<String>
  }
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
    sources.push(PactSource::Broker {
      url: url.to_string(),
      auth,
      consumers: matches.values_of("consumer-names").map(|v| v.map(ToString::to_string)
        .collect_vec()).unwrap_or_default(),
      providers: matches.values_of("provider-names").map(|v| v.map(ToString::to_string)
        .collect_vec()).unwrap_or_default()
    });
  }
  sources
}

async fn handle_command_args(args: Vec<String>) -> Result<(), ExitCode> {
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
        Err(ExitCode::from(3))
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
          .collect::<Vec<_>>();
        let interactions: usize = pacts.iter().map(|p| p.interactions.len()).sum();
        info!("Loaded {} pacts ({} total interactions)", pacts.len(), interactions);
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
    .arg(Arg::new("consumer-names")
      .long("consumer-names")
      .requires("broker-url")
      .takes_value(true)
      .multiple_values(true)
      .help("Consumer names to use to filter the Pacts fetched from the Pact broker"))
    .arg(Arg::new("provider-names")
      .long("provider-names")
      .requires("broker-url")
      .takes_value(true)
      .multiple_values(true)
      .help("Provider names to use to filter the Pacts fetched from the Pact broker"))
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
