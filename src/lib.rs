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

use failure::Error;
use url::percent_encoding::{utf8_percent_encode, PATH_SEGMENT_ENCODE_SET};

#[derive(Debug, Clone)]
pub struct Client {
    client: reqwest::Client,
    url: reqwest::Url,
    session_id: String,
}
#[derive(Debug, Deserialize)]
struct HasValue<T> {
    value: T,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewSessionReply {
    session_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewSessionReq {
    desired_capabilities: serde_json::Value,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WdErrorVal {
    error: String,
    message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WdError {
    status: u64,
    value: WdErrorVal,
}

impl Client {
    pub fn new<U: reqwest::IntoUrl>(url: U, req: NewSessionReq) -> Result<Self, Error> {
        let url = url.into_url()?;
        let client = reqwest::Client::new();

        let mut res = client.post(url.join("session")?).json(&req).send()?;

        // eprintln!("Response: {:?}", res);

        if res.status().is_success() {
            let body: NewSessionReply = res.json()?;
            Ok(Client {
                client: client,
                url: url,
                session_id: body.session_id,
            })
        } else {
            let err: WdError = res.json()?;
            eprintln!("{}", err.value.message);
            bail!("Something bad: {:?} / {:?}", res, err);
        }
    }

    pub fn close(&self) -> Result<(), Error> {
        let uri = self.url.join(&format!(
            "session/{}",
            utf8_percent_encode(&self.session_id, PATH_SEGMENT_ENCODE_SET)
        ))?;

        self.execute(self.client.delete(uri))
    }

    pub fn visit(&self, url: &str) -> Result<(), Error> {
        let uri = self.url.join(&format!(
            "session/{}/url",
            utf8_percent_encode(&self.session_id, PATH_SEGMENT_ENCODE_SET)
        ))?;
        self.execute(self.client.post(uri).json(&json!({ "url": url })))
    }

    pub fn back(&self) -> Result<(), Error> {
        let uri = self.url.join(&format!(
            "session/{}/back",
            utf8_percent_encode(&self.session_id, PATH_SEGMENT_ENCODE_SET)
        ))?;
        self.execute(self.client.post(uri))
    }

    pub fn forward(&self) -> Result<(), Error> {
        let uri = self.url.join(&format!(
            "session/{}/forward",
            utf8_percent_encode(&self.session_id, PATH_SEGMENT_ENCODE_SET)
        ))?;
        self.execute(self.client.post(uri))
    }

    pub fn current_url(&self) -> Result<String, Error> {
        let uri = self.url.join(&format!(
            "session/{}/url",
            utf8_percent_encode(&self.session_id, PATH_SEGMENT_ENCODE_SET)
        ))?;
        self.execute(self.client.get(uri))
    }

    fn execute<R>(&self, req: reqwest::RequestBuilder) -> Result<R, Error>
    where
        R: for<'de> serde::Deserialize<'de>,
    {
        let mut res = req.send()?;
        if res.status().is_success() {
            let data: HasValue<R> = res.json()?;
            Ok(data.value)
        } else {
            let json: serde_json::Value = res.json()?;
            bail!("Something on close: {:?} / {:?}", res, json);
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        if let Err(e) = self.close() {
            warn!("Closing webdriver client: {:?}", e);
        }
    }
}

impl NewSessionReq {
    pub fn chrome() -> Self {
        NewSessionReq {
            desired_capabilities: json!({ "browserName": "chrome" }),
        }
    }
}
