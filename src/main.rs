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

#[macro_use] extern crate clap;
#[cfg(test)]
#[macro_use(expect)]
extern crate expectest;
extern crate http;
extern crate hyper;
extern crate hyper_tls;
extern crate tokio;
extern crate itertools;
#[macro_use] extern crate log;
#[macro_use] extern crate maplit;
#[macro_use] #[allow(unused_imports)] extern crate p_macro;
#[macro_use] extern crate pact_matching;
#[cfg(test)]
extern crate quickcheck;
#[cfg(test)]
extern crate rand;
extern crate serde_json;
extern crate simplelog;
extern crate base64;

use clap::{App, AppSettings, Arg, ArgMatches, ErrorKind};
use http::StatusCode;
use hyper::{Body, Request as HyperRequest, Response as HyperResponse, Server};
use hyper::Client;
use hyper_tls::HttpsConnector;
use hyper::rt::{Future, Stream};
use hyper::service::service_fn_ok;
use itertools::Itertools;
use log::LogLevelFilter;
use pact_matching::*;
use pact_matching::models::{Interaction, Pact, PactSpecification, Request, Response};
use simplelog::{Config, SimpleLogger, TermLogger};
use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use tokio::runtime::Runtime;
use base64::encode;

mod pact_support;

fn main() {
    match handle_command_args() {
        Ok(_) => (),
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

/// Source for loading pacts
#[derive(Debug, Clone)]
pub enum PactSource {
    /// Load the pact from a pact file
    File(String),
    /// Load all the pacts from a Directory
    Dir(String),
    /// Load the pact from a URL
    URL(String, Option<String>)
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
            PactSource::URL(s!(v), matches.value_of("user").map(|u| u.to_string()))
        }).collect::<Vec<PactSource>>()),
        None => ()
    };
    sources
}

fn walkdir(dir: &Path) -> io::Result<Vec<io::Result<Pact>>> {
    let mut pacts = vec![];
    debug!("Scanning {:?}", dir);
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            walkdir(&path)?;
        } else {
            pacts.push(Pact::read_pact(&path))
        }
    }
    Ok(pacts)
}

fn pact_from_url(url: String, user: &Option<String>, runtime: &mut Runtime) -> Result<Pact, String> {
    match url.parse::<hyper::Uri>() {
        Ok(uri) => {
            let https = HttpsConnector::new(4).unwrap();
            let mut req = HyperRequest::builder();
            req.uri(uri).method("GET");
            match user {
                Some(ref u) => { req.header("Authorization", format!("Basic {}", encode(u))); },
                None => ()
            }
            debug!("Executing Request to fetch pact from URL: {:?}", req);
            let client = Client::builder()
                .build::<_, hyper::Body>(https);
            let future = client
                .request(req.body(Body::empty()).unwrap())
                .map_err(|err| format!("Request failed - {}", err))
                .and_then(|res| {
                    if res.status().is_success() {
                        Ok(res)
                    } else {
                        Err(format!("Request failed - {}", res.status()))
                    }
                })
                .and_then(|res| res.into_body().concat2().map_err(|err| format!("Failed to read the request body - {}", err)))
                .and_then(move |body| {
                    let pact_json = serde_json::from_slice(&body)
                        .map_err(|err| format!("Failed to parse Pact JSON - {}", err))?;
                    let pact = Pact::from_json(&url, &pact_json);
                    debug!("Fetched Pact: {:?}", pact);
                    Ok(pact)
                });
            runtime.block_on(future)
        },
        Err(err) => Err(format!("Request failed - {}", err))
    }
}

fn load_pacts(sources: Vec<PactSource>, runtime: &mut Runtime) -> Vec<Result<Pact, String>> {
    sources.iter().flat_map(|s| {
        match s {
            &PactSource::File(ref file) => vec![Pact::read_pact(Path::new(&file))
                .map_err(|err| format!("Failed to load pact '{}' - {}", file, err))],
            &PactSource::Dir(ref dir) => match walkdir(Path::new(dir)) {
                Ok(ref pacts) => pacts.iter().map(|p| {
                        match p {
                            &Ok(ref pact) => Ok(pact.clone()),
                            &Err(ref err) => Err(format!("Failed to load pact from '{}' - {}", dir, err))
                        }
                    }).collect(),
                Err(err) => vec![Err(format!("Could not load pacts from directory '{}' - {}", dir, err))]
            },
            &PactSource::URL(ref url, ref user) => vec![pact_from_url(url.clone(), user, runtime)
                .map_err(|err| format!("Failed to load pact '{}' - {}", url, err))]
        }
    })
    .collect()
}

fn method_supports_payload(request: &Request) -> bool {
  match request.method.to_uppercase().as_str() {
    "POST" | "PUT" | "PATCH" => true,
    _ => false
  }
}

struct ServerHandler {
  sources: Arc<Vec<Pact>>,
  auto_cors: bool
}

impl ServerHandler {
    fn new(sources: Vec<Pact>, auto_cors: bool) -> ServerHandler {
        ServerHandler {
          sources: Arc::new(sources),
          auto_cors
        }
    }

    fn find_matching_request(&self, request: &Request) -> Result<Response, String> {
        let match_results = self.sources
          .iter()
          .flat_map(|pact| pact.interactions.clone())
          .map(|i| (i.clone(), pact_matching::match_request(i.request, request.clone())))
          .filter(|&(_, ref mismatches)| mismatches.iter().all(|mismatch|{
            match mismatch {
              &Mismatch::MethodMismatch { .. } => false,
              &Mismatch::PathMismatch { .. } => false,
              &Mismatch::QueryMismatch { .. } => false,
              &Mismatch::BodyMismatch { .. } => !(method_supports_payload(request) && request.body.is_present()),
              _ => true
            }
          }))
          .sorted_by(|a, b| Ord::cmp(&a.1.len(), &b.1.len()))
          .iter()
          .map(|&(ref i, _)| i)
          .cloned()
          .collect::<Vec<Interaction>>();

        if match_results.len() > 1 {
            warn!("Found more than one pact request for method {} and path '{}', using the first one",
                request.method, request.path);
        }

        match match_results.first() {
            Some(interaction) => Ok(pact_matching::generate_response(&interaction.response)),
            None => {
              if self.auto_cors && request.method.to_uppercase() == "OPTIONS" {
                Ok(Response {
                  headers: Some(hashmap!{
                    s!("Access-Control-Allow-Headers") => s!("authorization,Content-Type"),
                    s!("Access-Control-Allow-Methods") => s!("GET, HEAD, POST, PUT, DELETE, CONNECT, OPTIONS, TRACE, PATCH"),
                    s!("Access-Control-Allow-Origin") => s!("*")
                  }),
                  .. Response::default_response()
                })
              } else {
                Err(s!("No matching request found"))
              }
            }
        }
    }

    fn handle(&self, mut req: HyperRequest<Body>) -> HyperResponse<Body> {
        let request = pact_support::hyper_request_to_pact_request(&mut req);
        info!("\n===> Received request: {:?}", request);
        info!("                   body: '{}'\n", request.body.str_value());
        match self.find_matching_request(&request) {
            Ok(ref response) => pact_support::pact_response_to_hyper_response(response),
            Err(msg) => {
                warn!("{}, sending {}", msg, StatusCode::NOT_FOUND);
                HyperResponse::builder().status(StatusCode::NOT_FOUND).body(Body::empty()).unwrap()
            }
        }
    }
}

fn start_server(port: u16, sources: Vec<Pact>, auto_cors: bool, runtime: &mut Runtime) -> Result<(), i32> {
    let addr = ([0, 0, 0, 0], port).into();
    match Server::try_bind(&addr) {
        Ok(builder) => {
            let server = builder.http1_keepalive(false)
                .serve(move || {
                    let service_handler = ServerHandler::new(sources.clone(), auto_cors);
                    service_fn_ok(move |req| service_handler.handle(req))
                });
            info!("Server started on port {}", server.local_addr().port());
            runtime.block_on(server.map_err(|err| error!("could not start server: {}", err)))
                .map_err(|_| {
                    format!("error occurred scheduling server future on Tokio runtime");
                    2
                })
        },
        Err(err) => {
            error!("could not start server: {}", err);
            Err(1)
        }
    }
}

fn handle_command_args() -> Result<(), i32> {
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
            .help("User and password to use when fetching pacts from URLS in user:password form"))
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
          .help("Automatically respond to OPTIONS requests and return default CORS headers"));

    let matches = app.get_matches_safe();
    match matches {
        Ok(ref matches) => {
            let level = matches.value_of("loglevel").unwrap_or("info");
            setup_logger(level);
            let sources = pact_source(matches);

            let mut tokio_runtime = Runtime::new().unwrap();
            let pacts = load_pacts(sources, &mut tokio_runtime);
            if pacts.iter().any(|p| p.is_err()) {
                error!("There were errors loading the pact files.");
                for error in pacts.iter().filter(|p| p.is_err()).cloned().map(|e| e.unwrap_err()) {
                    error!("  - {}", error);
                }
                tokio_runtime.shutdown_now();
                Err(3)
            } else {
                let port = matches.value_of("port").unwrap_or("0").parse::<u16>().unwrap();
                start_server(port, pacts.iter().cloned().map(|p| p.unwrap()).collect(),
                             matches.is_present("cors"), &mut tokio_runtime)
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
    "none" => LogLevelFilter::Off,
    _ => LogLevelFilter::from_str(level).unwrap()
  };
  match TermLogger::init(log_level, Config::default()) {
    Err(_) => SimpleLogger::init(log_level, Config::default()).unwrap_or(()),
    Ok(_) => ()
  }
}

#[cfg(test)]
mod test;
