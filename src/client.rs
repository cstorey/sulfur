use failure::Error;
use std::fmt;
use url::percent_encoding::{utf8_percent_encode, PATH_SEGMENT_ENCODE_SET};

/// The representation of a webdriver session.
#[derive(Debug, Clone)]
pub struct Client {
    client: reqwest::Client,
    url: reqwest::Url,
    session_id: Option<String>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HasValue {
    value: serde_json::Value,
}

impl HasValue {
    fn parse<T: serde::de::DeserializeOwned>(&self) -> Result<T, Error> {
        Ok(serde_json::from_value(self.value.clone())?)
    }

    // does `self.value | .error` exist?
    fn is_okay(&self) -> bool {
        return true;
    }
}

/// The representation of a new session request, allowing specification
/// of capabilities explicitly.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NewSessionReq {
    pub(crate) capabilities: Capabilities,
}
/// A representation of the [Capabilities](https://developer.mozilla.org/en-US/docs/Web/WebDriver/Capabilities)
/// we would like from the browser.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub(crate) always_match: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewSessionResp {
    pub(crate) session_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WdErrorVal {
    message: String,
}

#[derive(Debug, Deserialize, Fail)]
#[serde(rename_all = "camelCase")]
struct WdError {
    value: WdErrorVal,
}

impl fmt::Display for WdError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self.value.message)
    }
}

/// This reprsesents a selector for finding elements within a page.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct By {
    using: String,
    value: String,
}

// See §11.2.1 Locator strategies
impl By {
    /// Returns a selector for finding element by a css expression.
    pub fn css<S: Into<String>>(expr: S) -> Self {
        By {
            using: "css selector".into(),
            value: expr.into(),
        }
    }
}

/// The abstract representation of an element on the current page.
#[derive(Debug, Deserialize, Clone)]
pub struct Element {
    #[serde(rename = "element-6066-11e4-a52e-4f735466cecf")]
    _id: String,
}

impl Element {
    fn id(&self) -> &str {
        &*self._id
    }
}

struct PathSeg<'a>(&'a str);

impl Client {
    /// Creates a new webdriver session with the specified capabilities.
    pub fn new<U: reqwest::IntoUrl>(url: U, capabilities: Capabilities) -> Result<Self, Error> {
        let client = reqwest::Client::new();
        Client::new_with_http(url, capabilities, client)
    }

    // Ie: chromedriver returns the sessionId as a top-level item, wheras geckodriver (and presumably others)
    // return it under value.

    // §8.1 Creating a new session

    pub(crate) fn new_with_http<U: reqwest::IntoUrl>(
        url: U,
        capabilities: Capabilities,
        client: reqwest::Client,
    ) -> Result<Self, Error> {
        let req = NewSessionReq { capabilities };
        let url = url.into_url()?;
        let body: NewSessionResp = execute(client.post(url.join("session")?).json(&req))?;

        info!("New session response: {:?}", body);

        Ok(Client {
            client: client,
            url: url,
            session_id: Some(body.session_id),
        })
    }

    // §8.1 Delete session

    /// Terminates the session, possibly closing the browser window.§
    pub fn close(&mut self) -> Result<(), Error> {
        if let Some(session_id) = self.session_id.as_ref() {
            let path = format!("session/{}", PathSeg(&session_id));
            execute(self.client.delete(self.url.join(&path)?))?;
        }
        self.session_id = None;
        Ok(())
    }

    // §9.1 Navigate To

    /// Tells the browser to open the given URL.
    pub fn visit(&self, url: &str) -> Result<(), Error> {
        let path = format!("session/{}/url", PathSeg(self.session()?));
        execute(
            self.client
                .post(self.url.join(&path)?)
                .json(&json!({ "url": url })),
        )
    }

    // §9.3 Back

    /// Navigates to the previous page in the browser's history, just like
    /// pressing the back button.
    pub fn back(&self) -> Result<(), Error> {
        let path = format!("session/{}/back", PathSeg(self.session()?));
        execute(self.client.post(self.url.join(&path)?).json(&json!({})))
    }

    // §9.4 Forward

    /// Navigates to the next page in the browser's history, just like
    /// pressing the back button.
    pub fn forward(&self) -> Result<(), Error> {
        let path = format!("session/{}/forward", PathSeg(self.session()?));
        execute(self.client.post(self.url.join(&path)?).json(&json!({})))
    }

    // §9.6 Get Title

    /// Fetches the current page's title as a string.
    pub fn title(&self) -> Result<String, Error> {
        let path = format!("session/{}/title", PathSeg(self.session()?));
        execute(self.client.get(self.url.join(&path)?))
    }

    // §9.2 Get Current URL

    /// Fetches the browser's current URL, as would be shown in the URL bar.
    pub fn current_url(&self) -> Result<String, Error> {
        let path = format!("session/{}/url", PathSeg(self.session()?));
        execute(self.client.get(self.url.join(&path)?))
    }

    // §11.2.2 Find Element

    /// Attempts to lookup a single element by the given selector. Fails if
    /// Either no elements are found, or more than one is found.
    pub fn find_element(&self, by: &By) -> Result<Element, Error> {
        let path = format!("session/{}/element", PathSeg(self.session()?));
        let req = self.client.post(self.url.join(&path)?).json(&by);
        let result = execute(req)?;

        Ok(result)
    }

    // §11.2.3 Find Elements

    /// Attempts to lookup multiple elements by the given selector. May
    /// return zero or more.
    pub fn find_elements(&self, by: &By) -> Result<Vec<Element>, Error> {
        let path = format!("session/{}/elements", PathSeg(self.session()?));
        let req = self.client.post(self.url.join(&path)?).json(by);
        let result = execute(req)?;

        Ok(result)
    }

    // §11.2.4 Find Element From Element

    /// Find a single element relative to start element `elt` with the selector.
    /// Fails if zero or more than one are found.
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

    /// Attempts to lookup multiple elements relative to the start element
    /// `elt` by the given selector. May return zero or more.
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

    /// Get the contained text content from the given element, including
    /// that from child elementes.
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

    /// Fetch the tag name of the given element.
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

    /// Simulates clicking on the specified element.
    pub fn click(&self, elt: &Element) -> Result<(), Error> {
        let path = format!(
            "session/{}/element/{}/click",
            PathSeg(self.session()?),
            PathSeg(elt.id())
        );
        execute(self.client.post(self.url.join(&path)?).json(&json!({})))?;

        Ok(())
    }

    // §11.4.3 Element Send Keys

    /// Simulates typing into the given element, such as a text input.
    pub fn send_keys(&self, elt: &Element, keys: &'static str) -> Result<(), Error> {
        let url = self.url.join(&format!(
            "session/{}/element/{}/value",
            PathSeg(self.session()?),
            PathSeg(elt.id())
        ))?;
        execute(self.client.post(url).json(&json!({
            "text": keys,
            "value": [keys],
        })))?;

        Ok(())
    }
    // §11.4.2 Element Clear

    /// Clears the given element, such as an input field.
    pub fn clear(&self, elt: &Element) -> Result<(), Error> {
        let url = self.url.join(&format!(
            "session/{}/element/{}/clear",
            PathSeg(self.session()?),
            PathSeg(elt.id())
        ))?;
        execute(self.client.post(url).json(&json!({})))?;

        Ok(())
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
        if data.is_okay() {
            Ok(data)
        } else {
            let value: WdErrorVal = data.parse()?;
            Err(WdError { value: value }.into())
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
