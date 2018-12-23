//! Convenience functionality to to make managing the lifetime of a driver
//! and a client session easier.

use std::ops::{Deref, DerefMut};

use failure::Error;

use client;

/// Marker trait to mark something as a driver.
pub trait Driver {
    /// Shut down the driver.
    fn close(&mut self) -> Result<(), Error>;
}

/// This is designed to serve as a placeholder to make it easy to have the
/// driver live as long as the client.
pub struct DriverHolder {
    pub(crate) client: client::Client,
    // This is only used so we can drop it _after_ we have dropped the client.
    #[allow(dead_code)]
    pub(crate) driver: Box<Driver>,
}

impl DriverHolder {
    /// This will shut down both the associated webdriver session, and driver.
    pub fn close(self) -> Result<(), Error> {
        let DriverHolder {
            mut client,
            mut driver,
        } = self;
        client.close()?;
        driver.close()?;
        Ok(())
    }
}

impl Deref for DriverHolder {
    type Target = client::Client;
    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl DerefMut for DriverHolder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client
    }
}
