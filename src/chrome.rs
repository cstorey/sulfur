use client::{Capabilities, Client, NewSessionReq};
use failure::Error;
use junk_drawer::unused_port_no;
use reqwest;
use std::process::{Child, Command};
use std::{thread, time};

use failure::ResultExt;

pub struct Driver {
    child: Child,
    port: u16,
    http: reqwest::Client,
}

#[derive(Clone, Default)]
pub struct Config {
    headless: bool,
}

impl Driver {
    pub fn start() -> Result<Self, Error> {
        let http = reqwest::Client::new();
        let port = unused_port_no()?;
        debug!("Spawning chrome driver on port: {:?}", port);
        let mut cmd = Command::new("chromedriver");
        cmd.arg(format!("--port={}", port));
        // cmd.arg("--silent");
        // cmd.arg("--verbose");
        debug!("Starting command: {:?}", cmd);
        let child = cmd.spawn().context("Spawning chrome")?;

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

    pub fn new_session(&self) -> Result<Client, Error> {
        self.new_session_config(&Default::default())
    }
    pub fn new_session_config(&self, config: &Config) -> Result<Client, Error> {
        info!("Starting new session from instance at {}", self.port);
        let client =
            Client::new_with_http(&self.url(), config.to_new_session(), self.http.clone())?;
        Ok(client)
    }

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

impl Config {
    pub fn headless(&mut self, headless: bool) -> &mut Self {
        self.headless = headless;
        self
    }

    fn to_new_session(&self) -> NewSessionReq {
        let mut args = vec![];
        if self.headless {
            args.push("--headless")
        }
        NewSessionReq {
            capabilities: Capabilities {
                always_match: json!({
                "browserName": "chrome",
                "goog:chromeOptions" : {
                    "w3c" : true,
                    "args": args,
                }
             }),
            },
        }
    }
}
