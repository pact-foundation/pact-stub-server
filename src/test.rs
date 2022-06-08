use clap::ErrorKind;
use expectest::prelude::*;
use quickcheck::{quickcheck, TestResult};
use rand::Rng;

use crate::build_args;

use super::{integer_value, regex_value};

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
  let app = build_args("test", "0.0.0");
  let result = app.try_get_matches_from(args).unwrap_err();
  pretty_assertions::assert_eq!(result.kind(), ErrorKind::DisplayHelp);

  let expected_help = r#"test 0.0.0
Pact Stub Server

USAGE:
    test [OPTIONS]

OPTIONS:
    -b, --broker-url <broker-url>
            URL of the pact broker to fetch pacts from [env: PACT_BROKER_BASE_URL=]

        --cors-referer
            Set the CORS Access-Control-Allow-Origin header to the Referer

    -d, --dir <dir>
            Directory of pact files to load (can be repeated)

    -e, --extension <ext>
            File extension to use when loading from a directory (default is json)

        --empty-provider-state
            Include empty provider states when filtering with --provider-state

    -f, --file <file>
            Pact file to load (can be repeated)

    -h, --help
            Print help information

        --insecure-tls
            Disables TLS certificate validation

    -l, --loglevel <loglevel>
            Log level (defaults to info) [possible values: error, warn, info, debug, trace, none]

    -o, --cors
            Automatically respond to OPTIONS requests and return default CORS headers

    -p, --port <port>
            Port to run on (defaults to random port assigned by the OS)

        --provider-state-header-name <provider-state-header-name>
            Name of the header parameter containing the provider state to be used in case multiple
            matching interactions are found

    -s, --provider-state <provider-state>
            Provider state regular expression to filter the responses by

    -t, --token <token>
            Bearer token to use when fetching pacts from URLS or Pact Broker

    -u, --url <url>
            URL of pact file to fetch (can be repeated)

        --user <user>
            User and password to use when fetching pacts from URLS or Pact Broker in user:password
            form

    -v, --version
            Print version information
"#;
  pretty_assertions::assert_eq!(result.to_string(), expected_help);
}
