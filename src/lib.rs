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
pub mod driver;
pub mod gecko;

pub use client::*;
