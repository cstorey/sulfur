use failure::Error;
use std::fmt;
use url::percent_encoding::{utf8_percent_encode, PATH_SEGMENT_ENCODE_SET};

#[derive(Debug, Clone)]
pub struct Client {
    client: reqwest::Client,
    url: reqwest::Url,
    session_id: Option<String>,
}
#[derive(Debug, Deserialize)]
struct HasValue {
    status: u64,
    value: serde_json::Value,
}

impl HasValue {
    fn parse<T: serde::de::DeserializeOwned>(&self) -> Result<T, Error> {
        Ok(serde_json::from_value(self.value.clone())?)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewSessionReply {
    session_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewSessionReq {
    pub(crate) desired_capabilities: serde_json::Value,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WdErrorVal {
    message: String,
}

#[derive(Debug, Deserialize, Fail)]
#[serde(rename_all = "camelCase")]
struct WdError {
    status: u64,
    value: WdErrorVal,
}

impl fmt::Display for WdError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self.value.message)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct By {
    using: String,
    value: String,
}

impl By {
    pub fn css<S: Into<String>>(expr: S) -> Self {
        By {
            using: "css selector".into(),
            value: expr.into(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Element {
    #[serde(rename = "ELEMENT")]
    id: String,
}

struct PathSeg<'a>(&'a str);

impl Client {
    pub fn new<U: reqwest::IntoUrl>(url: U, req: NewSessionReq) -> Result<Self, Error> {
        let client = reqwest::Client::new();
        Client::new_with_http(url, req, client)
    }

    pub fn new_with_http<U: reqwest::IntoUrl>(
        url: U,
        req: NewSessionReq,
        client: reqwest::Client,
    ) -> Result<Self, Error> {
        let url = url.into_url()?;
        let mut res = client.post(url.join("session")?).json(&req).send()?;

        // eprintln!("Response: {:?}", res);

        if res.status().is_success() {
            let body: NewSessionReply = res.json()?;
            Ok(Client {
                client: client,
                url: url,
                session_id: Some(body.session_id),
            })
        } else {
            let err: WdError = res.json()?;
            eprintln!("{}", err.value.message);
            bail!("Something bad: {:?} / {:?}", res, err);
        }
    }

    pub fn close(&mut self) -> Result<(), Error> {
        if let Some(session_id) = self.session_id.as_ref() {
            let path = format!("session/{}", PathSeg(&session_id));
            execute(self.client.delete(self.url.join(&path)?))?;
        }
        self.session_id = None;
        Ok(())
    }

    pub fn visit(&self, url: &str) -> Result<(), Error> {
        let path = format!("session/{}/url", PathSeg(self.session()?));
        execute(
            self.client
                .post(self.url.join(&path)?)
                .json(&json!({ "url": url })),
        )
    }

    pub fn back(&self) -> Result<(), Error> {
        let path = format!("session/{}/back", PathSeg(self.session()?));
        execute(self.client.post(self.url.join(&path)?))
    }

    pub fn forward(&self) -> Result<(), Error> {
        let path = format!("session/{}/forward", PathSeg(self.session()?));
        execute(self.client.post(self.url.join(&path)?))
    }

    pub fn current_url(&self) -> Result<String, Error> {
        let path = format!("session/{}/url", PathSeg(self.session()?));
        execute(self.client.get(self.url.join(&path)?))
    }

    pub fn find_element(&self, by: &By) -> Result<Element, Error> {
        let path = format!("session/{}/element", PathSeg(self.session()?));
        let req = self.client.post(self.url.join(&path)?).json(by);
        let result = execute(req)?;

        Ok(result)
    }
    pub fn find_elements(&self, by: &By) -> Result<Vec<Element>, Error> {
        let path = format!("session/{}/elements", PathSeg(self.session()?));
        let req = self.client.post(self.url.join(&path)?).json(by);
        let result = execute(req)?;

        Ok(result)
    }

    pub fn find_element_from(&self, elt: &Element, by: &By) -> Result<Element, Error> {
        let path = format!(
            "session/{}/element/{}/element",
            PathSeg(self.session()?),
            PathSeg(&elt.id)
        );
        let req = self.client.post(self.url.join(&path)?).json(by);
        let result = execute(req)?;

        Ok(result)
    }
    pub fn find_elements_from(&self, elt: &Element, by: &By) -> Result<Vec<Element>, Error> {
        let path = format!(
            "session/{}/element/{}/elements",
            PathSeg(self.session()?),
            PathSeg(&elt.id)
        );
        let req = self.client.post(self.url.join(&path)?).json(by);
        let result = execute(req)?;

        Ok(result)
    }
    pub fn text(&self, elt: &Element) -> Result<String, Error> {
        let path = format!(
            "session/{}/element/{}/text",
            PathSeg(self.session()?),
            elt.id
        );
        let req = self.client.get(self.url.join(&path)?);
        let result = execute(req)?;

        Ok(result)
    }

    fn session(&self) -> Result<&str, Error> {
        return self
            .session_id
            .as_ref()
            .map(|r| &**r)
            .ok_or_else(|| failure::err_msg("No current session"));
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        if let Err(e) = self.close() {
            warn!("Closing webdriver client: {:?}", e);
        }
    }
}

impl<'a> fmt::Display for PathSeg<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let &PathSeg(ref val) = self;
        write!(
            fmt,
            "{}",
            utf8_percent_encode(&val, PATH_SEGMENT_ENCODE_SET)
        )
    }
}

fn execute_unparsed(req: reqwest::RequestBuilder) -> Result<HasValue, Error>
where
{
    let mut res = req.send()?;
    if res.status().is_success() {
        let data: HasValue = res.json()?;
        if data.status == 0 {
            Ok(data)
        } else {
            let value: WdErrorVal = data.parse()?;
            Err(WdError {
                status: data.status,
                value: value,
            }.into())
        }
    } else {
        let json: serde_json::Value = res.json()?;
        bail!("Something on close: {:?} / {:?}", res, json);
    }
}

fn execute<R>(req: reqwest::RequestBuilder) -> Result<R, Error>
where
    R: for<'de> serde::Deserialize<'de>,
{
    Ok(execute_unparsed(req)?.parse()?)
}
