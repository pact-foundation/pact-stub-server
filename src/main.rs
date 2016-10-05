//! Pact Stub Server

#![warn(missing_docs)]

#[macro_use] extern crate clap;
#[macro_use] extern crate p_macro;
#[macro_use] extern crate log;
#[macro_use] extern crate maplit;
#[macro_use] extern crate pact_matching;
extern crate simplelog;
extern crate hyper;
extern crate rustc_serialize;

#[cfg(test)]
#[macro_use(expect)]
extern crate expectest;

#[cfg(test)]
extern crate rand;

#[cfg(test)]
extern crate quickcheck;

use std::env;
use clap::{Arg, App, AppSettings, ErrorKind, ArgMatches};
use log::LogLevelFilter;
use simplelog::TermLogger;
use std::str::FromStr;
use hyper::server::{Handler, Server, Request as HyperRequest, Response as HyperResponse};
use hyper::client::Client;
use hyper::status::StatusCode;
use pact_matching::models::{PactSpecification, Pact, Interaction, Request, Response};
use std::sync::Arc;
use std::path::Path;
use std::io;
use std::fs;
use rustc_serialize::json::Json;

mod pact_support;

fn main() {
    match handle_command_args() {
        Ok(_) => (),
        Err(err) => std::process::exit(err)
    }
}

fn print_version() {
    println!("\npact stub server version  : v{}", crate_version!());
    println!("pact specification version: v{}", PactSpecification::V2.version_str());
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
    URL(String)
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
        Some(values) => sources.extend(values.map(|v| PactSource::URL(s!(v))).collect::<Vec<PactSource>>()),
        None => ()
    };
    sources
}

fn walkdir(dir: &Path) -> io::Result<Vec<io::Result<Pact>>> {
    let mut pacts = vec![];
    debug!("Scanning {:?}", dir);
    for entry in try!(fs::read_dir(dir)) {
        let entry = try!(entry);
        let path = entry.path();
        if path.is_dir() {
            try!(walkdir(&path));
        } else {
            pacts.push(Pact::read_pact(&path))
        }
    }
    Ok(pacts)
}

fn pact_from_url(url: &String) -> Result<Pact, String> {
    let client = Client::new();
    match client.get(url).send() {
        Ok(mut res) => if res.status.is_success() {
                let pact_json = Json::from_reader(&mut res);
                match pact_json {
                    Ok(ref json) => Ok(Pact::from_json(json)),
                    Err(err) => Err(format!("Failed to parse Pact JSON - {}", err))
                }
            } else {
                Err(format!("Request failed with status - {}", res.status))
            },
        Err(err) => Err(format!("Request failed - {}", err))
    }
}

fn load_pacts(sources: Vec<PactSource>) -> Vec<Result<Pact, String>> {
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
            &PactSource::URL(ref url) => vec![pact_from_url(url)
                .map_err(|err| format!("Failed to load pact '{}' - {}", url, err))]
        }
    })
    .collect()
}

fn match_request(expected_request: &Request, actual_request: &Request) -> bool {
    let mut mismatches = vec![];
    pact_matching::match_method(expected_request.method.clone(), actual_request.method.clone(),
        &mut mismatches);
    pact_matching::match_path(expected_request.path.clone(), actual_request.path.clone(),
        &mut mismatches, &expected_request.matching_rules);
    pact_matching::match_query(expected_request.query.clone(), actual_request.query.clone(),
        &mut mismatches, &expected_request.matching_rules);
    mismatches.is_empty()
}

struct ServerHandler {
    sources: Arc<Vec<Pact>>
}

impl ServerHandler {
    fn new(sources: Vec<Pact>) -> ServerHandler {
        ServerHandler {
            sources: Arc::new(sources)
        }
    }

    fn find_matching_request(&self, request: &Request) -> Result<Response, String> {
        let match_results = self.sources
            .iter()
            .flat_map(|pact| pact.interactions.clone())
            .filter(|i| match_request(&i.request, request))
            .collect::<Vec<Interaction>>();
        if match_results.len() > 1 {
            warn!("Found more than one pact request for path '{}', using the first one",
                request.path);
        }
        match match_results.first() {
            Some(interaction) => Ok(interaction.response.clone()),
            None => Err(s!("No matching request found"))
        }
    }
}

impl Handler for ServerHandler {

    fn handle(&self, mut req: HyperRequest, mut res: HyperResponse) {
        let request = pact_support::hyper_request_to_pact_request(&mut req);
        info!("Received request: {:?}", request);
        match self.find_matching_request(&request) {
            Ok(ref response) => pact_support::pact_response_to_hyper_response(res, response),
            Err(msg) => {
                warn!("{}", msg);
                *res.status_mut() = StatusCode::NotFound;
            }
        }
    }
}

fn start_server(port: u16, sources: Vec<Pact>) -> Result<(), i32> {
    match Server::http(format!("0.0.0.0:{}", port).as_str()) {
        Ok(mut server) => {
            server.keep_alive(None);
            match server.handle(ServerHandler::new(sources)) {
                Ok(listener) => {
                    info!("Server started on port {}", listener.socket.port());
                    Ok(())
                },
                Err(err) => {
                    error!("could not bind listener to port: {}", err);
                    Err(2)
                }
            }
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
            .help("Log level (defaults to warn)"))
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
        .arg(Arg::with_name("port")
            .short("p")
            .long("port")
            .takes_value(true)
            .use_delimiter(false)
            .help("Port to run on (defaults to random port assigned by the OS)")
            .validator(integer_value))
        ;

    let matches = app.get_matches_safe();
    match matches {
        Ok(ref matches) => {
            let level = matches.value_of("loglevel").unwrap_or("info");
            let log_level = match level {
                "none" => LogLevelFilter::Off,
                _ => LogLevelFilter::from_str(level).unwrap()
            };
            TermLogger::init(log_level).unwrap();
            let sources = pact_source(matches);
            let pacts = load_pacts(sources);
            if pacts.iter().any(|p| p.is_err()) {
                error!("There were errors loading the pact files.");
                for error in pacts.iter().filter(|p| p.is_err()).cloned().map(|e| e.unwrap_err()) {
                    error!("  - {}", error);
                }
                Err(3)
            } else {
                let port = matches.value_of("port").unwrap_or("0").parse::<u16>().unwrap();
                start_server(port, pacts.iter().cloned().map(|p| p.unwrap()).collect())
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
                    println!("");
                    Ok(())
                },
                _ => {
                    println!("{}", err.message);
                    err.exit()
                }
            }
        }
    }
}

#[cfg(test)]
mod test {

    use quickcheck::{TestResult, quickcheck};
    use rand::Rng;
    use super::integer_value;
    use expectest::prelude::*;

    #[test]
    fn validates_integer_value() {
        fn prop(s: String) -> TestResult {
            let mut rng = ::rand::thread_rng();
            if rng.gen() && s.chars().any(|ch| !ch.is_numeric()) {
                TestResult::discard()
            } else {
                let validation = integer_value(s.clone());
                match validation {
                    Ok(_) => TestResult::from_bool(!s.is_empty() && s.chars().all(|ch| ch.is_numeric() )),
                    Err(_) => TestResult::from_bool(s.is_empty() || s.chars().find(|ch| !ch.is_numeric() ).is_some())
                }
            }
        }
        quickcheck(prop as fn(_) -> _);

        expect!(integer_value(s!("1234"))).to(be_ok());
        expect!(integer_value(s!("1234x"))).to(be_err());
    }
}
