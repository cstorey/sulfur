use std::borrow::Cow;
use std::fmt;

use failure::Error;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};

const QUERY_ENCODE_SET: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');
const DEFAULT_ENCODE_SET: &AsciiSet = &QUERY_ENCODE_SET.add(b'`').add(b'?').add(b'{').add(b'}');
const PATH_SEGMENT_ENCODE_SET: &AsciiSet = &DEFAULT_ENCODE_SET.add(b'%').add(b'/');

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

/// Describes the timeouts used by the webserver service.

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Timeouts {
    /// Implicit timeout in milliseconds. Specifies how long the driver will
    /// wait for an element to be found, or for an element to be come interactive.
    pub implicit: u64,
    /// Page load timeout in milliseconds. Navigation will fail if a page load
    /// takes longer than this.
    pub page_load: u64,
    /// Script timeout in milliseconds. How long the implementation should
    /// wait for a script to run.
    pub script: u64,
}

/// Handle for a browser window.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Window(String);

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

// See §12.2.1 Locator strategies
impl By {
    // 11.2.1.1 CSS selectors
    /// Returns a selector for finding element by a css expression.
    pub fn css<S: Into<String>>(expr: S) -> Self {
        By {
            using: "css selector".into(),
            value: expr.into(),
        }
    }

    // 11.2.1.2 Link text
    /// Returns a selector for finding element link text
    pub fn link_text<S: Into<String>>(expr: S) -> Self {
        By {
            using: "link text".into(),
            value: expr.into(),
        }
    }

    // 11.2.1.3 Partial Link text
    /// Returns a selector for finding element link text
    pub fn partial_link_text<S: Into<String>>(expr: S) -> Self {
        By {
            using: "partial link text".into(),
            value: expr.into(),
        }
    }

    // 11.2.1.4 Tag name
    /// Returns a selector for finding element by tag name
    pub fn tag_name<S: Into<String>>(expr: S) -> Self {
        By {
            using: "tag name".into(),
            value: expr.into(),
        }
    }
    // 11.2.1.5 XPath
    /// Returns a selector for finding element by XPath
    pub fn xpath<S: Into<String>>(expr: S) -> Self {
        By {
            using: "xpath".into(),
            value: expr.into(),
        }
    }
}

/// The abstract representation of an element on the current page.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

    fn url_of_segments(&self, elts: &[&str]) -> Result<reqwest::Url, reqwest::UrlError> {
        let mut path = String::new();
        for (i, seg) in elts.iter().enumerate() {
            let enc: Cow<'_, str> = utf8_percent_encode(seg, PATH_SEGMENT_ENCODE_SET).into();
            if i > 0 {
                path.push('/')
            }
            path.push_str(&enc);
        }

        return self.url.join(&path);
    }

    // §8.2 Delete session

    /// Terminates the session, possibly closing the browser window.§
    pub fn close(&mut self) -> Result<(), Error> {
        if let Some(session_id) = self.session_id.as_ref() {
            let url = self.url_of_segments(&[&"session", &**session_id])?;
            execute(self.client.delete(url))?;
        }
        self.session_id = None;
        Ok(())
    }

    // §8.4 Get Timeouts

    /// Read the current set of timeouts.
    pub fn timeouts(&self) -> Result<Timeouts, Error> {
        let url = self.url_of_segments(&[&"session", self.session()?, &"timeouts"])?;
        Ok(execute(self.client.get(url))?)
    }

    // §8.5 Set Timeouts

    /// Change the current set of timeouts.
    pub fn set_timeouts(&self, timeouts: &Timeouts) -> Result<(), Error> {
        let url = self.url_of_segments(&[&"session", self.session()?, &"timeouts"])?;
        Ok(execute(self.client.post(url).json(timeouts))?)
    }

    // §9.1 Navigate To

    /// Tells the browser to open the given URL.
    pub fn visit(&self, visit_url: &str) -> Result<(), Error> {
        let url = self.url_of_segments(&[&"session", self.session()?, &"url"])?;
        execute(self.client.post(url).json(&json!({ "url": visit_url })))
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

    // §9.5 Refresh

    /// Reloads the current page from the server, just like
    /// pressing the "refresh" button.
    pub fn refresh(&self) -> Result<(), Error> {
        let path = format!("session/{}/refresh", PathSeg(self.session()?));
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

    // §10.1 Get Current Window handle

    /// Fetches the active window handle
    pub fn window(&self) -> Result<Window, Error> {
        let path = format!("session/{}/window", PathSeg(self.session()?));
        execute(self.client.get(self.url.join(&path)?))
    }

    // §10.2 Close Window

    /// Closes the _current_ window.
    pub fn close_window(&self) -> Result<Vec<Window>, Error> {
        let path = format!("session/{}/window", PathSeg(self.session()?));
        execute(self.client.delete(self.url.join(&path)?))
    }

    // §10.3 Switch to Window

    /// Switches to the given browser window / tab.
    pub fn switch_to_window(&self, window: &Window) -> Result<(), Error> {
        let path = format!("session/{}/window", PathSeg(self.session()?));
        let body = json!({
            "handle": window,
        });
        execute(self.client.post(self.url.join(&path)?).json(&body))
    }

    // §10.4 Get Current Window handles

    /// Lists all window handles.
    pub fn windows(&self) -> Result<Vec<Window>, Error> {
        let path = format!("session/{}/window/handles", PathSeg(self.session()?));
        execute(self.client.get(self.url.join(&path)?))
    }

    // §10.5 Switch to frame

    /// Switch to the frame by element reference
    pub fn switch_to_frame(&self, frame: Option<&Element>) -> Result<(), Error> {
        let path = format!("session/{}/frame", PathSeg(self.session()?));
        execute(
            self.client
                .post(self.url.join(&path)?)
                .json(&json!({ "id": frame })),
        )
    }

    /// Switch to the parent frame
    pub fn switch_to_parent_frame(&self) -> Result<(), Error> {
        let path = format!("session/{}/frame/parent", PathSeg(self.session()?));
        execute(self.client.post(self.url.join(&path)?).json(&json!({})))
    }

    // §12.2.2 Find Element

    /// Attempts to lookup a single element by the given selector. Fails if
    /// Either no elements are found, or more than one is found.
    pub fn find_element(&self, by: &By) -> Result<Element, Error> {
        let path = format!("session/{}/element", PathSeg(self.session()?));
        let req = self.client.post(self.url.join(&path)?).json(&by);
        let result = execute(req)?;

        Ok(result)
    }

    // §12.2.3 Find Elements

    /// Attempts to lookup multiple elements by the given selector. May
    /// return zero or more.
    pub fn find_elements(&self, by: &By) -> Result<Vec<Element>, Error> {
        let path = format!("session/{}/elements", PathSeg(self.session()?));
        let req = self.client.post(self.url.join(&path)?).json(by);
        let result = execute(req)?;

        Ok(result)
    }

    // §12.2.4 Find Element From Element

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

    // §12.2.5 Find Elements From Element

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

    // §12.3.5 Get Element Text

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

    // §12.3.2 Get Element Attribute

    /// Fetch the attribute value name of the given element.
    pub fn attribute(&self, elt: &Element, attribute: &str) -> Result<Option<String>, Error> {
        let path = format!(
            "session/{}/element/{}/attribute/{}",
            PathSeg(self.session()?),
            PathSeg(elt.id()),
            PathSeg(attribute),
        );
        let req = self.client.get(self.url.join(&path)?);
        let result = execute(req)?;

        Ok(result)
    }

    // §12.3.6 Get Element Tag Name

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

    // §12.4.1 Element Click

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

    // §12.4.3 Element Send Keys

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
    // §12.4.2 Element Clear

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
        let content_type = res
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        if content_type.starts_with("application/json") {
            let json: serde_json::Value = res.json()?;
            bail!("Error on execution: {:?} / {:?}", res, json);
        } else if content_type.starts_with("text/") {
            let message = res.text()?;
            bail!("Error on execution: {:?} / {:?}", res, message);
        } else {
            bail!("Error on execution: {:?}", res);
        }
    }
}

fn execute<R>(req: reqwest::RequestBuilder) -> Result<R, Error>
where
    R: for<'de> serde::Deserialize<'de>,
{
    Ok(execute_unparsed(req)?.parse()?)
}
