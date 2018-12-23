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
#[macro_use]
extern crate failure_derive;
extern crate url;
#[macro_use]
extern crate log;

mod junk_drawer;

pub mod chrome;
mod client;
mod driver;
pub mod gecko;

pub use client::*;
pub use driver::*;
