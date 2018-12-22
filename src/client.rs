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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewSessionReq {
    pub(crate) desired_capabilities: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewSessionResp {
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
    #[serde(rename = "element-6066-11e4-a52e-4f735466cecf")]
    _id2: Option<String>,
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

    /* New session geckodriver:
    {"value":{"sessionId":"dcd61912-edb7-b842-a480-625f1fa45826","capabilities":{"acceptInsecureCerts":false,"browserName":"firefox","browserVersion":"58.0.1","moz:accessibilityChecks":false,"moz:geckodriverVersion":"0.23.0","moz:headless":false,"moz:processID":6809,"moz:profile":"/var/folders/13/0cgwy88x5p1916xjmpw1fhc40000gn/T/rust_mozprofile.5zyna67LuJNh","moz:webdriverClick":true,"pageLoadStrategy":"normal","platformName":"darwin","platformVersion":"18.2.0","rotatable":false,"timeouts":{"implicit":0,"pageLoad":300000,"script":30000}}}}
    */

    /* `New session chromedriver:
        {"sessionId":"a90810adc57f5d6781a12f9b8735407d","status":0,"value":{"acceptInsecureCerts":false,"acceptSslCerts":false,"applicationCacheEnabled":false,"browserConnectionEnabled":false,"browserName":"chrome","chrome":{"chromedriverVersion":"2.45.615355 (d5698f682d8b2742017df6c81e0bd8e6a3063189)","userDataDir":"/var/folders/13/0cgwy88x5p1916xjmpw1fhc40000gn/T/.org.chromium.Chromium.BJONPd"},"cssSelectorsEnabled":true,"databaseEnabled":false,"goog:chromeOptions":{"debuggerAddress":"localhost:53115"},"handlesAlerts":true,"hasTouchScreen":false,"javascriptEnabled":true,"locationContextEnabled":true,"mobileEmulationEnabled":false,"nativeEvents":true,"networkConnectionEnabled":false,"pageLoadStrategy":"normal","platform":"Mac OS X","proxy":{},"rotatable":false,"setWindowRect":true,"strictFileInteractability":false,"takesHeapSnapshot":true,"takesScreenshot":true,"timeouts":{"implicit":0,"pageLoad":300000,"script":30000},"unexpectedAlertBehaviour":"ignore","version":"71.0.3578.98","webStorageEnabled":true}}
    */

    // Ie: chromedriver returns the sessionId as a top-level item, wheras geckodriver (and presumably others)
    // return it under value.


    // §8.1 Creating a new session
    pub fn new_with_http<U: reqwest::IntoUrl>(
        url: U,
        req: NewSessionReq,
        client: reqwest::Client,
    ) -> Result<Self, Error> {
        let url = url.into_url()?;
        let body : NewSessionResp = execute(client.post(url.join("session")?).json(&req))?;

        info!("New session response: {:?}", body);

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
        let req = self.client.post(self.url.join(&path)?).json(&by);
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
        if data.is_okay() {
            Ok(data)
        } else {
            let value: WdErrorVal = data.parse()?;
            Err(
                WdError {
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
