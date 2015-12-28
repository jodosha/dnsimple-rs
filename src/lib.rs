const BASE_URL: &'static str = "https://api.dnsimple.com/v2";

extern crate hyper;
pub mod dnsimple;

#[cfg(test)]
extern crate log;
#[cfg(test)]
#[macro_use]
extern crate yup_hyper_mock;
