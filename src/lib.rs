extern crate reqwest;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate webdriver;
#[macro_use]
extern crate failure;

use failure::Error;
use webdriver::capabilities::SpecNewSessionParameters;

pub struct Driver {
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

impl Driver {
    pub fn new<U: reqwest::IntoUrl>(url: U, caps: SpecNewSessionParameters) -> Result<Self, Error> {
        let url = url.into_url()?;
        let client = reqwest::Client::new();

        let req = json!({
            "capabilities": caps,
        });

        let mut res = client.post(url.join("session")?).json(&req).send()?;

        eprintln!("Response: {:?}", res);

        if res.status().is_success() {
            let body: HasValue<NewSessionReply> = res.json()?;
            Ok(Driver {
                client: client,
                url: url,
                session_id: body.value.session_id,
            })
        } else {
            let json: serde_json::Value = res.json()?;
            bail!("Something bad: {:?} / {:?}", res, json);
        }
    }

    pub fn close(self) -> Result<(), Error> {
        let uri = self.url.join(&format!("session/{}", self.session_id))?;
        let mut res = self.client.delete(uri).send()?;
        if res.status().is_success() {
            Ok(())
        } else {
            let json: serde_json::Value = res.json()?;
            bail!("Something on close: {:?} / {:?}", res, json);
        }
    }
}

pub fn chrome() -> SpecNewSessionParameters {
    let mut caps = webdriver::capabilities::SpecNewSessionParameters::default();
    let mut cap = webdriver::capabilities::Capabilities::default();
    cap.insert("browserName".to_string(), json!("chrome"));

    caps.firstMatch.push(cap);

    caps
}
