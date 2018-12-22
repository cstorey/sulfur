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
#[serde(rename_all = "camelCase")]
struct HasValue {
    status: u64,
    // If we find that anything other than Chromedriver doesn't
    // support this, we'll need to revise how we handle `execute` below.
    session_id: String,
    value: serde_json::Value,
}

impl HasValue {
    fn parse<T: serde::de::DeserializeOwned>(&self) -> Result<T, Error> {
        Ok(serde_json::from_value(self.value.clone())?)
    }
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

// See §11.2.1 Locator strategies
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
    _id: String,
}

impl Element {
    fn id(&self) -> &str {
        &*self._id
    }
}

struct PathSeg<'a>(&'a str);

impl Client {
    pub fn new<U: reqwest::IntoUrl>(url: U, req: NewSessionReq) -> Result<Self, Error> {
        let client = reqwest::Client::new();
        Client::new_with_http(url, req, client)
    }

    // §8.1 Creating a new session
    pub fn new_with_http<U: reqwest::IntoUrl>(
        url: U,
        req: NewSessionReq,
        client: reqwest::Client,
    ) -> Result<Self, Error> {
        let url = url.into_url()?;
        let body = execute_unparsed(client.post(url.join("session")?).json(&req))?;

        Ok(Client {
            client: client,
            url: url,
            session_id: Some(body.session_id),
        })
    }
    // §8.1 Delete session
    pub fn close(&mut self) -> Result<(), Error> {
        if let Some(session_id) = self.session_id.as_ref() {
            let path = format!("session/{}", PathSeg(&session_id));
            execute(self.client.delete(self.url.join(&path)?))?;
        }
        self.session_id = None;
        Ok(())
    }

    // §9.1 Navigate To
    pub fn visit(&self, url: &str) -> Result<(), Error> {
        let path = format!("session/{}/url", PathSeg(self.session()?));
        execute(self.client.post(self.url.join(&path)?).json(&json!({
            "url": url
        })))
    }
    // §9.3 Back
    pub fn back(&self) -> Result<(), Error> {
        let path = format!("session/{}/back", PathSeg(self.session()?));
        execute(self.client.post(self.url.join(&path)?))
    }

    // §9.4 Forward
    pub fn forward(&self) -> Result<(), Error> {
        let path = format!("session/{}/forward", PathSeg(self.session()?));
        execute(self.client.post(self.url.join(&path)?))
    }

    // §9.6 Get Title
    pub fn title(&self) -> Result<String, Error> {
        let path = format!("session/{}/title", PathSeg(self.session()?));
        execute(self.client.get(self.url.join(&path)?))
    }
    // §9.2 Get Current URL
    pub fn current_url(&self) -> Result<String, Error> {
        let path = format!("session/{}/url", PathSeg(self.session()?));
        execute(self.client.get(self.url.join(&path)?))
    }

    // §11.2.2 Find Element
    pub fn find_element(&self, by: &By) -> Result<Element, Error> {
        let path = format!("session/{}/element", PathSeg(self.session()?));
        let req = self.client.post(self.url.join(&path)?).json(by);
        let result = execute(req)?;

        Ok(result)
    }

    // §11.2.3 Find Elements
    pub fn find_elements(&self, by: &By) -> Result<Vec<Element>, Error> {
        let path = format!("session/{}/elements", PathSeg(self.session()?));
        let req = self.client.post(self.url.join(&path)?).json(by);
        let result = execute(req)?;

        Ok(result)
    }

    // §11.2.4 Find Element From Element
    pub fn find_element_from(&self, elt: &Element, by: &By) -> Result<Element, Error> {
        let path = format!(
            "session/{}/element/{}/element",
            PathSeg(self.session()?),
            PathSeg(elt.id())
        );
        let req = self.client.post(self.url.join(&path)?).json(by);
        let result = execute(req)?;

        Ok(result)
    }

    // §11.2.5 Find Elements From Element
    pub fn find_elements_from(&self, elt: &Element, by: &By) -> Result<Vec<Element>, Error> {
        let path = format!(
            "session/{}/element/{}/elements",
            PathSeg(self.session()?),
            PathSeg(elt.id())
        );
        let req = self.client.post(self.url.join(&path)?).json(by);
        let result = execute(req)?;

        Ok(result)
    }

    // §11.3.5 Get Element Text
    pub fn text(&self, elt: &Element) -> Result<String, Error> {
        let path = format!(
            "session/{}/element/{}/text",
            PathSeg(self.session()?),
            PathSeg(elt.id())
        );
        let req = self.client.get(self.url.join(&path)?);
        let result = execute(req)?;

        Ok(result)
    }

    // §11.3.6 Get Element Tag Name
    pub fn name(&self, elt: &Element) -> Result<String, Error> {
        let path = format!(
            "session/{}/element/{}/name",
            PathSeg(self.session()?),
            PathSeg(elt.id())
        );
        let req = self.client.get(self.url.join(&path)?);
        let result = execute(req)?;

        Ok(result)
    }

    // §11.4.1 Element Click
    pub fn click(&self, elt: &Element) -> Result<(), Error> {
        let path = format!(
            "session/{}/element/{}/click",
            PathSeg(self.session()?),
            PathSeg(elt.id())
        );
        execute(self.client.post(self.url.join(&path)?))?;

        Ok(())
    }

    // §11.4.3 Element Send Keys
    pub fn send_keys(&self, elt: &Element, keys: &'static str) -> Result<(), Error> {
        let url = self.url.join(&format!(
            "session/{}/element/{}/value",
            PathSeg(self.session()?),
            PathSeg(elt.id())
        ))?;
        execute(self.client.post(url).json(&json!({
            "value": [keys]
        })))?;

        Ok(())
    }
    // §11.4.2 Element Clear
    pub fn clear(&self, elt: &Element) -> Result<(), Error> {
        let url = self.url.join(&format!(
            "session/{}/element/{}/clear",
            PathSeg(self.session()?),
            PathSeg(elt.id())
        ))?;
        execute(self.client.post(url))?;

        Ok(())
    }
    fn session(&self) -> Result<&str, Error> {
        return self.session_id.as_ref().map(|r| &**r).ok_or_else(|| {
            failure::err_msg("No current session")
        });
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
            Err(
                WdError {
                    status: data.status,
                    value: value,
                }.into(),
            )
        }
    } else {
        let json: serde_json::Value = res.json()?;
        bail!("Error on execution: {:?} / {:?}", res, json);
    }
}

fn execute<R>(req: reqwest::RequestBuilder) -> Result<R, Error>
where
    R: for<'de> serde::Deserialize<'de>,
{
    Ok(execute_unparsed(req)?.parse()?)
}
