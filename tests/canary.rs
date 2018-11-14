extern crate env_logger;
extern crate sulfur;
#[macro_use]
extern crate lazy_static;
extern crate futures;
extern crate tokio;
extern crate warp;
#[macro_use]
extern crate log;

use std::net::SocketAddr;
use std::sync::Mutex;

use futures::sync::oneshot;
use sulfur::*;
use tokio::runtime;

const TEST_HTML_DIR: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/html");

lazy_static! {
    static ref DRIVER: ChromeDriver = ChromeDriver::start().expect("ChromeDriver::start");
    static ref RT: Mutex<runtime::Runtime> =
        Mutex::new(runtime::Runtime::new().expect("tokio runtime"));
    static ref SERVER: TestServer = {
        debug!("Starting test server for {}", TEST_HTML_DIR);
        let srv = TestServer::start(warp::fs::dir(TEST_HTML_DIR));
        debug!("Test server at {}", srv.url());
        srv
    };
}

#[test]
fn can_run_chromedriver() {
    env_logger::try_init().unwrap_or_default();
    let mut s = DRIVER.new_session().expect("new_session");
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

    let mut s = DRIVER.new_session().expect("new_session");

    s.visit(&url).expect("visit");

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
fn find_text_present() {
    env_logger::try_init().unwrap_or_default();

    let url = SERVER.url();
    let s = DRIVER.new_session().expect("new_session");

    s.visit(&url).expect("visit");
    let elt = s.find_element(&By::css("#an-id")).expect("find #an-id");
    println!("Elt: {:?}", elt);
    let text_content = s.text(&elt).expect("read text");
    assert_eq!(text_content.trim(), "Hello world");
}

#[test]
fn find_multiple_elements() {
    env_logger::try_init().unwrap_or_default();

    let url = SERVER.url();
    let s = DRIVER.new_session().expect("new_session");

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
    let s = DRIVER.new_session().expect("new_session");

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
    let s = DRIVER.new_session().expect("new_session");

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
