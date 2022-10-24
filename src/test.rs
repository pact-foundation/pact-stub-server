use clap::crate_version;
use clap::error::ErrorKind;
use expectest::prelude::*;
use quickcheck::{quickcheck, TestResult};
use rand::Rng;

use crate::build_args;

use super::{integer_value, regex_value};

#[test]
fn verify_cli() {
  let app = build_args();
  app.debug_assert();
}

#[test]
fn validates_integer_value() {
    fn prop(s: String) -> TestResult {
        let mut rng = ::rand::thread_rng();
        if rng.gen() && s.chars().any(|ch| !ch.is_numeric()) {
            TestResult::discard()
        } else {
            let validation = integer_value(s.as_str());
            match validation {
                Ok(_) => TestResult::from_bool(!s.is_empty() && s.chars().all(|ch| ch.is_numeric() )),
                Err(_) => TestResult::from_bool(s.is_empty() || s.chars().find(|ch| !ch.is_numeric() ).is_some())
            }
        }
    }
    quickcheck(prop as fn(_) -> _);

    expect!(integer_value("1234")).to(be_ok());
    expect!(integer_value("1234x")).to(be_err());
}

#[test]
fn validates_regex_value() {
    expect!(regex_value("1234")).to(be_ok());
    expect!(regex_value("[")).to(be_err());
}

#[test]
fn test_default_args() {
  let args = vec!["test".to_string(), "--help".to_string()];
  let app = build_args();
  let result = app.try_get_matches_from(args).unwrap_err();
  pretty_assertions::assert_eq!(result.kind(), ErrorKind::DisplayHelp);

  let expected_help = format!(r#"Pact Stub Server {}

Usage: test [OPTIONS]

Options:
  -l, --loglevel <loglevel>
          Log level (defaults to info) [default: info] [possible values: error, warn, info, debug, trace, none]
  -f, --file <file>
          Pact file to load (can be repeated)
  -d, --dir <dir>
          Directory of pact files to load (can be repeated)
  -e, --extension <ext>
          File extension to use when loading from a directory (default is json)
  -u, --url <url>
          URL of pact file to fetch (can be repeated)
  -b, --broker-url <broker-url>
          URL of the pact broker to fetch pacts from [env: PACT_BROKER_BASE_URL=]
      --user <user>
          User and password to use when fetching pacts from URLS or Pact Broker in user:password form
  -t, --token <token>
          Bearer token to use when fetching pacts from URLS or Pact Broker
  -p, --port <port>
          Port to run on (defaults to random port assigned by the OS)
  -o, --cors
          Automatically respond to OPTIONS requests and return default CORS headers
      --cors-referer
          Set the CORS Access-Control-Allow-Origin header to the Referer
      --insecure-tls
          Disables TLS certificate validation
  -s, --provider-state <provider-state>
          Provider state regular expression to filter the responses by
      --provider-state-header-name <provider-state-header-name>
          Name of the header parameter containing the provider state to be used in case multiple matching interactions are found
      --empty-provider-state
          Include empty provider states when filtering with --provider-state
      --consumer-names <consumer-names>
          Consumer names to use to filter the Pacts fetched from the Pact broker
      --provider-names <provider-names>
          Provider names to use to filter the Pacts fetched from the Pact broker
  -v, --version
          Print version information
  -h, --help
          Print help information
"#, crate_version!());
  pretty_assertions::assert_eq!(result.to_string(), expected_help);
}
