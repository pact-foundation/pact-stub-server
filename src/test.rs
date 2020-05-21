use quickcheck::{TestResult, quickcheck};
use rand::Rng;
use super::{integer_value, regex_value};
use expectest::prelude::*;
use pact_matching::s;

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

#[test]
fn validates_regex_value() {
    expect!(regex_value(s!("1234"))).to(be_ok());
    expect!(regex_value(s!("["))).to(be_err());
}
