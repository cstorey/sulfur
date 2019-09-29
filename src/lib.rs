#![deny(missing_docs)]

//! Sulfur provides an implementation of the webdriver protocol,
//! used for remote controlling a browser, as well as functionality for
//! conveniently running a browser locally.

extern crate reqwest;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate serde;
#[macro_use]
extern crate failure;
extern crate url;
#[macro_use]
extern crate log;
extern crate base64;
extern crate percent_encoding;
extern crate rand;

mod junk_drawer;

pub mod chrome;
mod client;
mod driver;
pub mod gecko;

pub use client::*;
pub use driver::*;
