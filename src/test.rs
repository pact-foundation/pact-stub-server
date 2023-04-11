use expectest::prelude::*;

use crate::build_args;

use super::{integer_value, regex_value};

#[test]
fn verify_cli() {
  let app = build_args();
  app.debug_assert();
}

#[test]
fn validates_integer_value() {
    expect!(integer_value("1234")).to(be_ok().value(1234));
    expect!(integer_value("1234x")).to(be_err());
}

#[test]
fn validates_regex_value() {
    expect!(regex_value("1234")).to(be_ok());
    expect!(regex_value("\\d+")).to(be_ok());
    expect!(regex_value("[")).to(be_err());
}
