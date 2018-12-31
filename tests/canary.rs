extern crate env_logger;
extern crate sulfur;
#[macro_use]
extern crate lazy_static;
extern crate futures;
extern crate tokio;
extern crate warp;
#[macro_use]
extern crate log;
extern crate failure;
extern crate url;

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::{thread, time};

use futures::sync::oneshot;

use sulfur::chrome;
use sulfur::*;
use tokio::runtime;

const TEST_HTML_DIR: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/html");

lazy_static! {
    static ref RT: Mutex<runtime::Runtime> =
        Mutex::new(runtime::Runtime::new().expect("tokio runtime"));
    static ref SERVER: TestServer = {
        debug!("Starting test server for {}", TEST_HTML_DIR);
        let srv = TestServer::start(warp::fs::dir(TEST_HTML_DIR));
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

// ... Oh god. Maybe I should just use warp.
// At least I can make that work.
// https://github.com/seanmonstar/warp/blob/master/examples/returning.rs
// https://github.com/seanmonstar/warp/blob/master/examples/dir.rs
impl TestServer {
    fn start<S, R>(f: S) -> Self
    where
        S: warp::Filter<Extract = (R,), Error = warp::Rejection> + Sync + Send + 'static,
        R: warp::Reply,
    {
        let (tx, rx) = oneshot::channel::<()>();

        let (addr, server) = warp::serve(f).bind_with_graceful_shutdown(([127, 0, 0, 1], 0), rx);

        RT.lock().expect("lock runtime").spawn(server);

        TestServer {
            drop: Some(tx),
            addr: addr,
        }
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
    )
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
