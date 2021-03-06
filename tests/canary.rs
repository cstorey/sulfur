extern crate env_logger;
extern crate sulfur;
#[macro_use]
extern crate lazy_static;
extern crate futures;
extern crate tokio;
#[macro_use]
extern crate log;
extern crate failure;
extern crate hyper;
extern crate hyper_staticfile;
extern crate tempfile;
extern crate url;

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Mutex;
use std::{thread, time};

use futures::channel::oneshot;
use futures::future::select;
use hyper::service::make_service_fn;
use tokio::runtime;

use sulfur::chrome;
use sulfur::*;

const TEST_HTML_DIR: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/html");

lazy_static! {
    static ref RT: Mutex<runtime::Runtime> =
        Mutex::new(runtime::Runtime::new().expect("tokio runtime"));
    static ref SERVER: TestServer = {
        debug!("Starting test server for {}", TEST_HTML_DIR);
        let srv = TestServer::start(Path::new(TEST_HTML_DIR)).expect("Testserver");
        debug!("Test server at {}", srv.url());
        srv
    };
}

fn new_session() -> Result<DriverHolder, failure::Error> {
    let driver = env::var("DRIVER").unwrap_or_else(|e| {
        warn!("$DRIVER not specified, using chromedriver: {:?}", e);
        "chromedriver".into()
    });
    match &*driver {
        "geckodriver" => {
            info!("Starting instance with {:?}", driver);
            let driver = gecko::start(gecko::Config::default().headless(true))?;
            Ok(driver)
        }
        "chromedriver" | _ => {
            info!("Starting instance with {:?}", driver);
            let driver = chrome::start(chrome::Config::default().headless(true))?;
            Ok(driver)
        }
    }
}

#[test]
fn can_run_driver() {
    env_logger::try_init().unwrap_or_default();
    let s = new_session().expect("new_session");
    s.close().expect("close");
}

struct TestServer {
    drop: Option<oneshot::Sender<()>>,
    addr: SocketAddr,
}

impl TestServer {
    fn start(path: &Path) -> Result<Self, failure::Error> {
        use std::net;

        let (tx, rx) = oneshot::channel::<()>();
        let path = path.to_owned();
        let addr: net::SocketAddr = "127.0.0.1:0".parse()?;
        let sock = net::TcpListener::bind(&addr)?;
        let addr = sock.local_addr()?;

        let content = hyper_staticfile::Static::new(&path);
        let make_service =
            make_service_fn(move |_| futures::future::ok::<_, hyper::Error>(content.clone()));

        thread::Builder::new()
            .name("TestServer".to_string())
            .spawn(move || {
                let mut rt = RT.lock().expect("lock runtime");
                rt.block_on(async {
                    let srv = hyper::Server::from_tcp(sock)
                        .expect("listen on socket")
                        .serve(make_service);
                    debug!("Starting server");
                    select(srv, rx).await
                })
            })?;

        let s = TestServer {
            drop: Some(tx),
            addr: addr,
        };
        debug!("Test server listening at: {}", s.url());
        Ok(s)
    }
    fn url(&self) -> String {
        format!("http://{}:{}/", self.addr.ip(), self.addr.port())
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(tx) = self.drop.take() {
            tx.send(()).expect("Send shutdown signal");
        }
    }
}

#[test]
fn can_navigate() {
    env_logger::try_init().unwrap_or_default();

    let url = SERVER.url();

    let s = new_session().expect("new_session");

    s.visit(&url).expect("visit");

    let () = s
        .click(
            &s.find_element(&By::css(".clickable-link"))
                .expect("find .clickable-link"),
        )
        .expect("click");

    let main_page = s.current_url().expect("current_url");
    assert!(
        main_page.starts_with(&url),
        "current URL: {:?} should start with {:?}",
        main_page,
        url
    );

    s.back().expect("back");

    let current = s.current_url().expect("current_url");

    assert!(
        current != main_page,
        "current URL: {:?} different from {:?}",
        current,
        main_page
    );

    s.refresh().expect("refresh page");
    let current2 = s.current_url().expect("current_url");
    assert_eq!(
        current, current2,
        "current URL: {:?} still on {:?}",
        current2, current
    );

    s.forward().expect("back");

    let current = s.current_url().expect("current_url");
    assert!(
        current == main_page,
        "current URL: {:?} back on {:?}",
        current,
        main_page
    );

    s.close().expect("close")
}

#[test]
fn can_load_title() {
    env_logger::try_init().unwrap_or_default();

    let url = SERVER.url();

    let s = new_session().expect("new_session");

    s.visit(&url).expect("visit");

    let title = s.title().expect("current_url");
    assert_eq!(title, "Page title");
}

#[test]
fn find_element_fails_on_missing_element() {
    env_logger::try_init().unwrap_or_default();

    let url = SERVER.url();
    let s = new_session().expect("new_session");

    s.visit(&url).expect("visit");
    let res = s.find_element(&By::css("#i-do-not-exist"));
    assert!(res.is_err(), "Result should be an error: {:?}", res);
}

#[test]
fn find_text_present() {
    env_logger::try_init().unwrap_or_default();

    let url = SERVER.url();
    let s = new_session().expect("new_session");

    s.visit(&url).expect("visit");
    let elt = s.find_element(&By::css("#an-id")).expect("find #an-id");
    println!("Elt: {:?}", elt);
    let text_content = s.text(&elt).expect("read text");
    assert_eq!(text_content.trim(), "Hello world");
}

#[test]
fn find_via_link_text() {
    env_logger::try_init().unwrap_or_default();

    let url = SERVER.url();
    let s = new_session().expect("new_session");

    s.visit(&url).expect("visit");
    let elt = s
        .find_element(&By::link_text("Link target"))
        .expect("find Some link");
    println!("Elt: {:?}", elt);
    let text_content = s.text(&elt).expect("read text");
    assert_eq!(text_content.trim(), "Link target");
}

#[test]
fn find_via_partial_link_text() {
    env_logger::try_init().unwrap_or_default();

    let url = SERVER.url();
    let s = new_session().expect("new_session");

    s.visit(&url).expect("visit");
    let elt = s
        .find_element(&By::partial_link_text("ink targe"))
        .expect("find Some link");
    println!("Elt: {:?}", elt);
    let text_content = s.text(&elt).expect("read text");
    assert_eq!(text_content.trim(), "Link target");
}

#[test]
fn find_via_tag_name() {
    env_logger::try_init().unwrap_or_default();

    let url = SERVER.url();
    let s = new_session().expect("new_session");

    s.visit(&url).expect("visit");
    let elt = s.find_element(&By::tag_name("button")).expect("find");
    println!("Elt: {:?}", elt);
    let text_content = s.text(&elt).expect("read text");
    assert_eq!(text_content.trim(), "Go");
}

#[test]
fn find_via_xpath() {
    env_logger::try_init().unwrap_or_default();

    let url = SERVER.url();
    let s = new_session().expect("new_session");

    s.visit(&url).expect("visit");
    let elt = s
        .find_element(&By::xpath("//div[@id='with-children']/p[1]"))
        .expect("find");
    println!("Elt: {:?}", elt);
    let text_content = s.text(&elt).expect("read text");
    assert_eq!(text_content.trim(), "Hello world");
}

#[test]
fn find_tag_name() {
    env_logger::try_init().unwrap_or_default();

    let url = SERVER.url();
    let s = new_session().expect("new_session");

    s.visit(&url).expect("visit");
    let elt = s.find_element(&By::css("#an-id")).expect("find #an-id");
    println!("Elt: {:?}", elt);
    let tag_name = s.name(&elt).expect("read tag name");
    assert_eq!(tag_name, "p");
}

#[test]
fn find_attribute_value() {
    env_logger::try_init().unwrap_or_default();

    let url = SERVER.url();
    let s = new_session().expect("new_session");

    s.visit(&url).expect("visit");
    let elt = s
        .find_element(&By::css("#find-attribute-value"))
        .expect("find #find-attribute-value");
    println!("Elt: {:?}", elt);
    let value = s
        .attribute(&elt, "data-my-id")
        .expect("read attribute value");
    assert_eq!(value, Some("my-id-value".to_string()));

    let value2 = s.attribute(&elt, "missing").expect("read missing value");
    assert_eq!(value2, None);
}

#[test]
fn find_multiple_elements() {
    env_logger::try_init().unwrap_or_default();

    let url = SERVER.url();
    let s = new_session().expect("new_session");

    s.visit(&url).expect("visit");
    let elts = s
        .find_elements(&By::css("#missing-element"))
        .expect("find #an-id");
    println!("Elt: {:?}", elts);
    assert!(elts.is_empty(), "Element {:?} should be None", elts);

    let elts = s
        .find_elements(&By::css(".three-of-these"))
        .expect("find .three-of-these");

    println!("Elt: {:?}", elts);
    assert!(
        elts.len() == 3,
        "Element {:?} should be have three items",
        elts
    )
}

#[test]
fn find_text_present_from_child() {
    env_logger::try_init().unwrap_or_default();

    let url = SERVER.url();
    let s = new_session().expect("new_session");

    s.visit(&url).expect("visit");
    let parent = s
        .find_element(&By::css("#with-children"))
        .expect("find #with-children");
    let elt = s
        .find_element_from(&parent, &By::css(".a-child"))
        .expect("find #an-id");
    println!("Elt: {:?}", elt);
    let text_content = s.text(&elt).expect("read text");
    assert_eq!(text_content.trim(), "Hello world");
}

#[test]
fn find_multiple_elements_from_child() {
    env_logger::try_init().unwrap_or_default();

    let url = SERVER.url();
    let s = new_session().expect("new_session");

    s.visit(&url).expect("visit");
    let parent = s
        .find_element(&By::css("#with-children"))
        .expect("find #with-children");
    let elts = s
        .find_elements_from(&parent, &By::css("#missing-element"))
        .expect("find #an-id");
    println!("Elt: {:?}", elts);
    assert!(elts.is_empty(), "Element {:?} should be None", elts);

    let elts = s
        .find_elements(&By::css(".three-of-these"))
        .expect("find .three-of-these");

    println!("Elt: {:?}", elts);
    assert!(
        elts.len() == 3,
        "Element {:?} should be have three items",
        elts
    )
}

#[test]
fn should_click_links() {
    env_logger::try_init().unwrap_or_default();

    let url = SERVER.url();
    let s = new_session().expect("new_session");
    s.visit(&url).expect("visit");
    let main_page = s.current_url().expect("current_url");
    let elt = s
        .find_element(&By::css(".clickable-link"))
        .expect("find #with-children");
    println!("Elt: {:?}", elt);
    let () = s.click(&elt).expect("click");
    let new_page = s.current_url().expect("current_url");

    assert_ne!(new_page, main_page);
}

#[test]
fn form_submission() {
    env_logger::try_init().unwrap_or_default();

    let url = SERVER.url();
    let s = new_session().expect("new_session");
    s.visit(&url).expect("visit");
    let text = s
        .find_element(&By::css("#the-form input[type='text']"))
        .expect("find text");
    let () = s.send_keys(&text, "Canary text").expect("send_keys");

    let button = s
        .find_element(&By::css("#the-form button"))
        .expect("find button");
    let () = s.click(&button).expect("click");
    let url = s.current_url().expect("current_url");
    let url = url::Url::parse(&url).expect("parse url");
    let q = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect::<BTreeMap<_, _>>();

    assert_eq!(
        q.get("text"),
        Some(&"Canary text".to_string()),
        "Query text:{:?} from URL {:?}",
        q.get("text"),
        url
    )
}

#[test]
fn form_element_clearing() {
    env_logger::try_init().unwrap_or_default();

    let url = SERVER.url();
    let s = new_session().expect("new_session");
    s.visit(&url).expect("visit");
    let text = s
        .find_element(&By::css("#the-form input[type='text']"))
        .expect("find text");

    let button = s
        .find_element(&By::css("#the-form button"))
        .expect("find button");

    s.send_keys(&text, "Canary text").expect("send_keys");
    s.clear(&text).expect("clear");
    s.click(&button).expect("click");

    let url = s.current_url().expect("current_url");
    let url = url::Url::parse(&url).expect("parse url");
    let q = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect::<BTreeMap<_, _>>();

    assert_eq!(
        q.get("text"),
        Some(&"".to_string()),
        "Query text:{:?} from URL {:?}",
        q.get("text"),
        url
    )
}

#[test]
fn timeouts() {
    env_logger::try_init().unwrap_or_default();

    let url = SERVER.url();
    let s = new_session().expect("new_session");
    s.visit(&url).expect("visit");

    s.set_timeouts(&Timeouts {
        ..Timeouts::default()
    })
    .expect("set timeouts");

    let _t = s.timeouts().expect("get timeouts");
}

#[test]
fn window_handles() {
    env_logger::try_init().unwrap_or_default();

    let url = SERVER.url();
    let s = new_session().expect("new_session");
    s.visit(&url).expect("visit");

    let main_window = s.window().expect("get window");
    let known_windows = s.windows().expect("get windows");

    assert_eq!(vec![main_window.clone()], known_windows);

    let opener_link = s
        .find_element(&By::css(".new-window"))
        .expect("find_element");

    s.click(&opener_link).expect("click link");

    let known = known_windows.iter().cloned().collect::<BTreeSet<_>>();
    wait_until(time::Duration::from_secs(10), || {
        let current = s.windows()?.into_iter().collect::<BTreeSet<_>>();
        Ok(current != known)
    })
    .expect("Wait for window open");

    let known_windows = s.windows().expect("get windows");
    assert_eq!(2, known_windows.len());
    let other_window = known_windows
        .iter()
        .cloned()
        .find(|w| w != &main_window)
        .expect("other window");

    // Yes, we switch to the current window. This would be easier if
    // `/session/{session}/window/new` was supported anywhere but the w3c spec.
    s.switch_to_window(&other_window).expect("switch to window");
    let other_window2 = s.window().expect("get window");
    assert_eq!(other_window, other_window2);

    let other_url = s.current_url().expect("current_url");

    assert!(
        other_url.contains("#new-window"),
        "New window URL should contain `#new-window`, was: {:?}",
        other_url,
    );

    let new_handles = s.close_window().expect("close window");
    assert_eq!(vec![main_window.clone()], new_handles);
}

#[test]
fn frames_by_ref() {
    env_logger::try_init().unwrap_or_default();

    let url = SERVER.url();
    let s = new_session().expect("new_session");
    s.visit(&url).expect("visit");

    let content = s
        .find_elements(&By::css("#inner-content"))
        .expect("find inner content");
    assert_eq!(
        Vec::<Element>::new(),
        content,
        "Finding element within an iframe should yield no item"
    );

    let iframe = s.find_element(&By::css("iframe")).expect("find iframe");
    s.switch_to_frame(Some(&iframe)).expect("switch to frame");
    let content = s
        .find_elements(&By::css("#inner-content"))
        .expect("find inner content");
    assert_eq!(
        1,
        content.len(),
        "Looking for #inner-content in iframe: saw {:?}",
        content
    );

    s.switch_to_frame(None).expect("switch to default");
    let content = s
        .find_elements(&By::css("#inner-content"))
        .expect("find inner content");
    assert_eq!(
        Vec::<Element>::new(),
        content,
        "Looking for #inner-content in iframe: saw {:?}",
        content
    )
}

#[test]
fn frames_parent() {
    env_logger::try_init().unwrap_or_default();

    let url = SERVER.url();
    let s = new_session().expect("new_session");
    s.visit(&url).expect("visit");

    let iframe = s.find_element(&By::css("iframe")).expect("find iframe");
    s.switch_to_frame(Some(&iframe)).expect("switch to frame");
    let content = s
        .find_elements(&By::css("#inner-content"))
        .expect("find inner content");
    assert_eq!(
        1,
        content.len(),
        "Looking for #inner-content in iframe: saw {:?}",
        content
    );

    s.switch_to_parent_frame().expect("switch to parent");
    let content = s
        .find_elements(&By::css("#inner-content"))
        .expect("find inner content");
    assert_eq!(
        Vec::<Element>::new(),
        content,
        "Looking for #inner-content in top frame, should be empty"
    )
}

#[test]
fn should_include_message_in_errors() {
    env_logger::try_init().unwrap_or_default();

    let url = SERVER.url();
    let s = new_session().expect("new_session");

    s.visit(&url).expect("visit");
    let err = s
        .find_element(&By::tag_name("thing-that-is-not-present"))
        .expect_err("failing find");
    let wd_error = err.downcast_ref::<WdError>().expect("Extract WdError");
    assert!(
        wd_error.message.contains("thing-that-is-not-present"),
        "Error contains the name of the missing tag: {:?}",
        wd_error
    )
}

#[test]
fn should_get_page_source() {
    env_logger::try_init().unwrap_or_default();

    let url = SERVER.url();
    let s = new_session().expect("new_session");

    s.visit(&url).expect("visit");
    let source = s.page_source().expect("page_source");
    let expected = "<title>Page title</title>";
    assert!(
        source.contains(expected),
        "Page source should contain {}: Got {:?}",
        expected,
        source,
    )
}

#[test]
fn should_get_document_screenshot() {
    use std::fs;
    use std::io::Write;

    env_logger::try_init().unwrap_or_default();

    let url = SERVER.url();
    let s = new_session().expect("new_session");
    s.visit(&url).expect("visit");

    let ss = s.screenshot().expect("document screenshot");

    assert!(ss.len() > 0, "Returns non-empty set of bytes");

    let path = tempfile::tempdir().expect("tempdir").into_path();
    let ss_path = path.join("document.png");
    let mut w = fs::File::create(&ss_path).expect("document.png");
    w.write_all(&ss).expect("write_all");
    w.flush().expect("flush");
    println!("Wrote {} bytes of image to {:?}", ss.len(), ss_path);
}

#[test]
fn should_get_element_screenshot() {
    use std::fs;
    use std::io::Write;

    env_logger::try_init().unwrap_or_default();

    let url = SERVER.url();
    let s = new_session().expect("new_session");
    s.visit(&url).expect("visit");

    let elt = s
        .find_element(&By::css(".clickable-link"))
        .expect("find .clickable-link");

    let ss = s.element_screenshot(&elt).expect("element screenshot");

    assert!(ss.len() > 0, "Returns non-empty set of bytes");

    let path = tempfile::tempdir().expect("tempdir").into_path();
    let ss_path = path.join("document.png");
    let mut w = fs::File::create(&ss_path).expect("document.png");
    w.write_all(&ss).expect("write_all");
    w.flush().expect("flush");
    println!("Wrote {} bytes of image to {:?}", ss.len(), ss_path);
}

fn wait_until<F: FnMut() -> Result<bool, failure::Error>>(
    deadline: time::Duration,
    mut check: F,
) -> Result<bool, failure::Error> {
    let mut pause_time = time::Duration::from_millis(1);
    let started_at = time::Instant::now();
    while started_at.elapsed() < deadline && !check()? {
        debug!("Pausing for {:?}", pause_time);
        thread::sleep(pause_time);
        pause_time *= 2;
    }

    Ok(check()?)
}
