//! # Standalone Pact Stub Server
//!
//! This project provides a server that can generate responses based on pact files. It is a single executable binary. It implements the [V4 Pact specification](https://github.com/pact-foundation/pact-specification/tree/version-4).
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
//! Pact Stub Server 0.5.2
//!
//! Usage: pact-stub-server [OPTIONS]
//!
//! Options:
//!   -l, --loglevel <loglevel>
//!           Log level (defaults to info) [default: info] [possible values: error, warn, info, debug, trace, none]
//!   -f, --file <file>
//!           Pact file to load (can be repeated)
//!   -d, --dir <dir>
//!           Directory of pact files to load (can be repeated)
//!   -e, --extension <ext>
//!           File extension to use when loading from a directory (default is json)
//!   -u, --url <url>
//!           URL of pact file to fetch (can be repeated)
//!   -b, --broker-url <broker-url>
//!           URL of the pact broker to fetch pacts from [env: PACT_BROKER_BASE_URL=]
//!       --user <user>
//!           User and password to use when fetching pacts from URLS or Pact Broker in user:password form
//!   -t, --token <token>
//!           Bearer token to use when fetching pacts from URLS or Pact Broker
//!   -p, --port <port>
//!           Port to run on (defaults to random port assigned by the OS)
//!   -o, --cors
//!           Automatically respond to OPTIONS requests and return default CORS headers
//!       --cors-referer
//!           Set the CORS Access-Control-Allow-Origin header to the Referer
//!       --insecure-tls
//!           Disables TLS certificate validation
//!   -s, --provider-state <provider-state>
//!           Provider state regular expression to filter the responses by
//!       --provider-state-header-name <provider-state-header-name>
//!           Name of the header parameter containing the provider state to be used in case multiple matching interactions are found
//!       --empty-provider-state
//!           Include empty provider states when filtering with --provider-state
//!      --consumer-name <consumer-name>
//!           Consumer name to use to filter the Pacts fetched from the Pact broker (can be repeated)
//!       --provider-name <provider-name>
//!           Provider name to use to filter the Pacts fetched from the Pact broker (can be repeated)
//!   -v, --version
//!           Print version information
//!   -h, --help
//!           Print help information
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
//! | `-b, --broker-url <broker-url>` | URL | Loads all the pacts from the Pact broker |
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
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::sync::mpsc::channel;

use clap::{Command, Arg, ArgMatches, ArgAction, command, crate_version};
use clap::error::ErrorKind;
use mimalloc::MiMalloc;
use pact_models::prelude::*;
use pact_models::prelude::v4::*;
use regex::Regex;
use tracing::{debug, error, info, warn};
use tracing_core::LevelFilter;
use tracing_subscriber::FmtSubscriber;
use tokio::sync::broadcast;
use notify::RecursiveMode;
use notify_debouncer_mini::{DebouncedEventKind, new_debouncer};
use crate::loading::load_pacts;

use crate::server::ServerHandler;

/// Setup file watcher for watch mode
fn setup_file_watcher(
  sources: Vec<PactSource>,
  matches: &ArgMatches,
  shared_pacts: Arc<Mutex<Vec<(V4Pact, PactSource)>>>,
  reload_tx: broadcast::Sender<()>
) {
  let watch_paths = get_watch_paths(&sources);
  if watch_paths.is_empty() {
    warn!("No file or directory sources found for watching");
    return;
  }

  let insecure_tls = matches.get_flag("insecure-tls");
  let ext = matches.get_one::<String>("ext").cloned();
  
  std::thread::spawn(move || {
    let (debounce_tx, debounce_rx) = channel();
    let mut debouncer = match new_debouncer(Duration::from_secs(1), debounce_tx) {
      Ok(debouncer) => debouncer,
      Err(e) => {
        error!("Failed to create file debouncer: {}", e);
        return;
      }
    };

    // Watch all file and directory sources
    for path in &watch_paths {
      if let Err(e) = debouncer.watcher().watch(path, RecursiveMode::Recursive) {
        error!("Failed to watch path {:?}: {}", path, e);
      } else {
        info!("Watching for changes in: {:?}", path);
      }
    }

    let runtime = tokio::runtime::Runtime::new().unwrap();
    
    loop {
      match debounce_rx.recv() {
        Ok(Ok(events)) => {
          for event in events.iter() {
            match &event.kind {
              DebouncedEventKind::Any => {
                info!("File change detected in watched directory");
                
                // Reload pacts
                let pacts_result = runtime.block_on(load_pacts(sources.clone(), insecure_tls, ext.as_ref()));
                if pacts_result.iter().any(|p| p.is_err()) {
                  error!("Error reloading pacts:");
                  for error in pacts_result.iter().filter_map(|p| p.as_ref().err()) {
                    error!("  - {}", error);
                  }
                } else {
                  let new_pacts = pacts_result.iter()
                    .filter_map(|result| result.as_ref().ok())
                    .map(|(p, s)| (p.as_v4_pact().unwrap(), s.clone()))
                    .collect::<Vec<_>>();
                  
                  let interactions: usize = new_pacts.iter().map(|(p, _)| p.interactions.len()).sum();
                  info!("Reloaded {} pacts ({} total interactions)", new_pacts.len(), interactions);
                  
                  *shared_pacts.lock().unwrap() = new_pacts;
                  let _ = reload_tx.send(());
                }
                break;
              }
              _ => {}
            }
          }
        }
        Ok(Err(e)) => {
          error!("Watch error: {:?}", e);
          break;
        }
        Err(e) => {
          error!("Debouncer channel error: {:?}", e);
          break;
        }
      }
    }
  });
}

/// Extract file and directory paths from pact sources for watching
fn get_watch_paths(sources: &[PactSource]) -> Vec<PathBuf> {
  sources.iter()
    .filter_map(|source| match source {
      PactSource::File(path) => Some(PathBuf::from(path)),
      PactSource::Dir(path) => Some(PathBuf::from(path)),
      _ => None, // URLs and Broker sources are not watchable
    })
    .collect()
}

mod pact_support;
mod server;
mod loading;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;


pub fn print_version() {
    println!("pact stub server version  : v{}", env!("CARGO_PKG_VERSION"));
    println!("pact specification version: v{}", PactSpecification::V4.version_str());
}

fn integer_value(v: &str) -> Result<u16, String> {
    v.parse::<u16>().map_err(|e| format!("'{}' is not a valid port value: {}", v, e) )
}

fn regex_value(v: &str) -> Result<Regex, String> {
  if v.is_empty() {
    Err("Regular expression is empty".to_string())
  } else {
    Regex::new(v).map_err(|e| format!("'{}' is not a valid regular expression: {}", v, e))
  }
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
    consumers: Vec<Regex>,
    /// Provider names to filter Pacts with
    providers: Vec<Regex>
  },
  /// Source that is not known, only used for unit testing
  Unknown
}

fn pact_source(matches: &ArgMatches) -> Vec<PactSource> {
  let mut sources = vec![];

  if let Some(values) = matches.get_many::<String>("file") {
    sources.extend(values.map(|v| PactSource::File(v.clone())).collect::<Vec<PactSource>>());
  }

  if let Some(values) = matches.get_many::<String>("dir") {
    sources.extend(values.map(|v| PactSource::Dir(v.clone())).collect::<Vec<PactSource>>());
  }

  if let Some(values) = matches.get_many::<String>("url") {
    sources.extend(values.map(|v| {
      let auth = matches.get_one::<String>("user")
        .map(|u| {
          let mut auth = u.split(':');
          HttpAuth::User(auth.next().unwrap().to_string(), auth.next().map(|p| p.to_string()))
        })
        .or_else(|| matches.get_one::<String>("token").map(|v| HttpAuth::Token(v.clone())));
      PactSource::URL(v.clone(), auth)
    }).collect::<Vec<PactSource>>());
  }

  if let Some(url) = matches.get_one::<String>("broker-url") {
    let auth = matches.get_one::<String>("user")
      .map(|u| {
        let mut auth = u.split(':');
        HttpAuth::User(auth.next().unwrap().to_string(), auth.next().map(|p| p.to_string()))
      })
      .or_else(|| matches.get_one::<String>("token").map(|v| HttpAuth::Token(v.clone())));
    debug!("Loading all pacts from Pact Broker at {} using {} authentication", url,
      auth.clone().map(|auth| auth.to_string()).unwrap_or_else(|| "no".to_string()));
    sources.push(PactSource::Broker {
      url: url.to_string(),
      auth,
      consumers: matches.get_many::<Regex>("consumer-name").unwrap_or_default().into_iter().cloned().collect(),
      providers: matches.get_many::<Regex>("provider-name").unwrap_or_default().into_iter().cloned().collect()
    });
  }

  sources
}

/// Handles the command line arguments and runs the stub server accordingly.
///
/// Used by the binary crate. Parses the provided arguments, sets up logging, loads pact files, and starts the server.
pub async fn handle_command_args(args: Vec<String>) -> Result<(), ExitCode> {
  let app = build_args();
  match app.try_get_matches_from(args) {
    Ok(results) => handle_matches(&results).await,

    Err(ref err) => match err.kind() {
        ErrorKind::DisplayHelp => {
            println!("{}", err);
            Ok(())
        }
        ErrorKind::DisplayVersion => {
            print_version();
            println!();
            Ok(())
        }
        _ => err.exit(),
    },
  }
}

/// Handles the command line arguments and runs the stub server accordingly.
///
/// Used by library consumers. Creates a new Tokio runtime, handle the matches
/// and starts the server.
pub fn process_stub_command(args: &ArgMatches) -> Result<(), ExitCode>  {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let res = handle_matches(args).await;
        match res {
            Ok(()) => Ok(()),
            Err(code) => Err(code),
        }
    })
}

async fn handle_matches(matches: &ArgMatches) -> Result<(), ExitCode> {
      let level = matches.get_one::<String>("loglevel").cloned()
        .unwrap_or_else(|| "info".to_string());
      setup_logger(level.as_str());
      let sources = pact_source(matches);
      let watch_mode = matches.get_flag("watch");

      let pacts = load_pacts(sources.clone(), matches.get_flag("insecure-tls"),
        matches.get_one("ext")).await;
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
        let port = *matches.get_one::<u16>("port").unwrap_or(&0);
        let provider_state = matches.get_one::<Regex>("provider-state").cloned();
        let provider_state_header_name = matches.get_one::<String>("provider-state-header-name").cloned();
        let empty_provider_states = matches.get_flag("empty-provider-state");
        let pacts = pacts.iter()
          .map(|result| {
            // Currently, as_v4_pact won't fail as it upgrades older formats to V4, so is safe to unwrap
            let (p, s) = result.as_ref().unwrap();
            (p.as_v4_pact().unwrap(), s.clone())
          })
          .collect::<Vec<_>>();
        let interactions: usize = pacts.iter().map(|(p, _)| p.interactions.len()).sum();
        info!("Loaded {} pacts ({} total interactions)", pacts.len(), interactions);
        let auto_cors = matches.get_flag("cors");
        let referer = matches.get_flag("cors-referer");
        
        if watch_mode {
          // Setup shared state for pacts when in watch mode
          let shared_pacts = Arc::new(Mutex::new(pacts.clone()));
          let (reload_tx, reload_rx) = broadcast::channel::<()>(1);
          
          // Setup file watching if in watch mode
          setup_file_watcher(sources, matches, shared_pacts.clone(), reload_tx.clone());
          
          let server_handler = ServerHandler::new_with_watch(
            shared_pacts,
            reload_tx,
            auto_cors,
            referer,
            provider_state,
            provider_state_header_name,
            empty_provider_states);
          tokio::task::spawn_blocking(move || {
            server_handler.start_server(port)
          }).await.unwrap()
        } else {
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
      }
}

/// Creates a new clap Command instance with the command line arguments for the stub server.
/// This function defines the command line interface for the stub server, including options for logging, pact file sources, and server configuration.
pub fn build_args() -> Command {
  command!()
    .about(format!("Pact Stub Server {}", crate_version!()))
    .arg_required_else_help(true)
    .disable_version_flag(true)
    .arg(Arg::new("loglevel")
      .short('l')
      .long("loglevel")
      .default_value("info")
      .value_parser(["error", "warn", "info", "debug", "trace", "none"])
      .help("Log level (defaults to info)"))
    .arg(Arg::new("file")
      .short('f')
      .long("file")
      .required_unless_present_any(&["dir", "url", "broker-url"])
      .action(ArgAction::Append)
      .value_parser(clap::builder::NonEmptyStringValueParser::new())
      .help("Pact file to load (can be repeated)"))
    .arg(Arg::new("dir")
      .short('d')
      .long("dir")
      .required_unless_present_any(&["file", "url", "broker-url"])
      .action(ArgAction::Append)
      .value_parser(clap::builder::NonEmptyStringValueParser::new())
      .help("Directory of pact files to load (can be repeated)"))
    .arg(Arg::new("ext")
      .short('e')
      .long("extension")
      .value_parser(clap::builder::NonEmptyStringValueParser::new())
      .requires("dir")
      .help("File extension to use when loading from a directory (default is json)"))
    .arg(Arg::new("url")
      .short('u')
      .long("url")
      .required_unless_present_any(&["file", "dir", "broker-url"])
      .action(ArgAction::Append)
      .value_parser(clap::builder::NonEmptyStringValueParser::new())
      .help("URL of pact file to fetch (can be repeated)"))
    .arg(Arg::new("broker-url")
      .short('b')
      .long("broker-url")
      .env("PACT_BROKER_BASE_URL")
      .required_unless_present_any(&["file", "dir", "url"])
      .value_parser(clap::builder::NonEmptyStringValueParser::new())
      .help("URL of the pact broker to fetch pacts from"))
    .arg(Arg::new("user")
      .long("user")
      .value_parser(clap::builder::NonEmptyStringValueParser::new())
      .conflicts_with("token")
      .help("User and password to use when fetching pacts from URLS or Pact Broker in user:password form"))
    .arg(Arg::new("token")
      .short('t')
      .long("token")
      .value_parser(clap::builder::NonEmptyStringValueParser::new())
      .conflicts_with("user")
      .help("Bearer token to use when fetching pacts from URLS or Pact Broker"))
    .arg(Arg::new("port")
      .short('p')
      .long("port")
      .use_value_delimiter(false)
      .help("Port to run on (defaults to random port assigned by the OS)")
      .value_parser(integer_value))
    .arg(Arg::new("cors")
      .short('o')
      .long("cors")
      .action(ArgAction::SetTrue)
      .help("Automatically respond to OPTIONS requests and return default CORS headers"))
    .arg(Arg::new("cors-referer")
      .long("cors-referer")
      .requires("cors")
      .action(ArgAction::SetTrue)
      .help("Set the CORS Access-Control-Allow-Origin header to the Referer"))
    .arg(Arg::new("insecure-tls")
      .long("insecure-tls")
      .action(ArgAction::SetTrue)
      .help("Disables TLS certificate validation"))
    .arg(Arg::new("provider-state")
      .short('s')
      .long("provider-state")
      .value_parser(regex_value)
      .help("Provider state regular expression to filter the responses by"))
    .arg(Arg::new("provider-state-header-name")
      .long("provider-state-header-name")
      .value_parser(clap::builder::NonEmptyStringValueParser::new())
      .help("Name of the header parameter containing the provider state to be used in case \
      multiple matching interactions are found"))
    .arg(Arg::new("empty-provider-state")
      .long("empty-provider-state")
      .requires("provider-state")
      .action(ArgAction::SetTrue)
      .help("Include empty provider states when filtering with --provider-state"))
    .arg(Arg::new("consumer-name")
      .long("consumer-name")
      .alias("consumer-names")
      .requires("broker-url")
      .action(ArgAction::Append)
      .value_parser(regex_value)
      .help("Consumer name or regex to use to filter the Pacts fetched from the Pact broker (can be repeated)"))
    .arg(Arg::new("provider-name")
      .long("provider-name")
      .alias("provider-names")
      .requires("broker-url")
      .action(ArgAction::Append)
      .value_parser(regex_value)
      .help("Provider name or regex to use to filter the Pacts fetched from the Pact broker (can be repeated)"))
    .arg(Arg::new("watch")
      .short('w')
      .long("watch")
      .action(ArgAction::SetTrue)
      .help("Watch for changes in pact files and reload automatically"))
    .arg(Arg::new("version")
      .short('v')
      .long("version")
      .action(ArgAction::Version)
      .help("Print version information"))
}

fn setup_logger(level: &str) {
  let log_level = match level {
    "none" => LevelFilter::OFF,
    _ => LevelFilter::from_str(level).unwrap_or(LevelFilter::INFO)
  };
  let subscriber = FmtSubscriber::builder()
    .compact()
    .with_max_level(log_level)
    .with_thread_names(true)
    .finish();
  if let Err(err) = tracing::subscriber::set_global_default(subscriber) {
    eprintln!("ERROR: Failed to initialise global tracing subscriber - {err}");
  };
}

#[cfg(test)]
mod test;
