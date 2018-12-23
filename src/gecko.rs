use std::process::{Child, Command};
use std::{thread, time};

use failure::Error;
use failure::ResultExt;
use reqwest;

use client::{Capabilities, Client};
use driver::{self, DriverHolder};
use junk_drawer::unused_port_no;

/// Represents a `geckodriver` process.
pub struct Driver {
    child: Child,
    port: u16,
    http: reqwest::Client,
}
/// Allows extra configuration for chrome instances.
#[derive(Clone, Default)]
pub struct Config {
    headless: bool,
}

/// Start a chromedriver instance, along with a new browser session.
pub fn start(config: &Config) -> Result<DriverHolder, Error> {
    let driver = Driver::start()?;
    let client = driver.new_session_config(config)?;
    Ok(DriverHolder {
        driver: Box::new(driver),
        client: client,
    })
}

impl Driver {
    /// Start a geckodriver instance on an automatically assigned port.
    pub fn start() -> Result<Self, Error> {
        let http = reqwest::Client::new();
        let port = unused_port_no()?;
        debug!("Spawning chrome driver on port: {:?}", port);
        let mut cmd = Command::new("geckodriver");
        cmd.arg(format!("--port={}", port));
        // cmd.arg("--silent");
        // cmd.arg("--verbose");
        debug!("Starting command: {:?}", cmd);
        let child = cmd.spawn().context("Spawning geckodriver")?;

        let mut driver = Driver { child, port, http };

        let mut pause_time = time::Duration::from_millis(1);
        while !driver.is_healthy() {
            driver.ensure_still_alive()?;
            debug!("Pausing for {:?}", pause_time);
            thread::sleep(pause_time);
            pause_time *= 2;
        }
        info!("Setup done! running on port {:?}", driver.port);

        Ok(driver)
    }

    /// Build a new webdriver session with default sessions.
    pub fn new_session(&self) -> Result<Client, Error> {
        self.new_session_config(&Default::default())
    }

    /// Build a new webdriver session with the specified configuration.
    pub fn new_session_config(&self, config: &Config) -> Result<Client, Error> {
        info!("Starting new session from instance at {}", self.port);
        let client =
            Client::new_with_http(&self.url(), config.to_capabilities(), self.http.clone())?;
        Ok(client)
    }

    /// Shut down the geckodriver process. This assumes that the session has
    /// been shut down seperately.
    pub fn close(&mut self) -> Result<(), Error> {
        self.child.kill()?;
        self.child.wait()?;
        Ok(())
    }

    fn url(&self) -> String {
        format!("http://127.0.0.1:{}/", self.port)
    }

    // §8.3 Status
    fn is_healthy(&self) -> bool {
        let url = format!("{}status", self.url());
        match self.http.get(&url).send() {
            Err(e) => {
                warn!("Could not fetch {}: {:?}", url, e);
                false
            }
            Ok(resp) => {
                debug!("Got {} -> {:?}", url, resp);
                resp.status().is_success()
            }
        }
    }

    fn ensure_still_alive(&mut self) -> Result<(), Error> {
        match self.child.try_wait()? {
            Some(status) => {
                warn!("child exited with {}", status);
                bail!("Child process failed: {:?}", status)
            }
            None => Ok(()),
        }
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        debug!("Dropping child");
        let _ = self.child.kill();
    }
}

impl driver::Driver for Driver {
    fn close(&mut self) -> Result<(), Error> {
        self.child.kill()?;
        Ok(())
    }
}

impl Config {
    /// Specifies if the firefox instance should be headless, or whether
    /// it should show the UI.
    pub fn headless(&mut self, headless: bool) -> &mut Self {
        self.headless = headless;
        self
    }

    fn to_capabilities(&self) -> Capabilities {
        let mut args = vec![];
        if self.headless {
            args.push("--headless")
        }
        Capabilities {
            always_match: json!({
               "browserName": "firefox",
               "moz:firefoxOptions": { "args": args },
            }),
        }
    }
}
