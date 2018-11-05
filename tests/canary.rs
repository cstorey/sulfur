extern crate env_logger;
extern crate sulfur;
#[macro_use]
extern crate lazy_static;
extern crate futures;
extern crate hyper;

use hyper::rt::Future;
use hyper::service::service_fn_ok;
use hyper::{Body, Request, Response, Server};
use std::thread;

use sulfur::*;

lazy_static! {
    static ref DRIVER: ChromeDriver = ChromeDriver::start().expect("ChromeDriver::start");
}

#[test]
fn can_run_chromedriver() {
    env_logger::try_init().unwrap_or_default();
    let mut s = DRIVER.new_session().expect("new_session");
    s.close().expect("close");
}

struct OnDrop<F: FnOnce()>(Option<F>);

impl<F: FnOnce()> Drop for OnDrop<F> {
    fn drop(&mut self) {
        if let Some(f) = self.0.take() {
            f()
        }
    }
}

#[test]
fn can_navigate() {
    env_logger::try_init().unwrap_or_default();

    const PHRASE: &str = "Hello, World!";

    fn hello_world(_req: Request<Body>) -> Response<Body> {
        Response::new(Body::from(PHRASE))
    }

    let server = Server::bind(&([127, 0, 0, 1], 0).into()).serve(|| service_fn_ok(hello_world));

    let (tx, rx) = futures::sync::oneshot::channel::<()>();

    let laddr = server.local_addr();

    let graceful = server
        .with_graceful_shutdown(rx)
        .map_err(|err| eprintln!("server error: {}", err));

    thread::spawn(move || hyper::rt::run(graceful)).expect("spawn server thread");
    let _dropper = OnDrop(Some(|| tx.send(()).expect("send")));

    let url = format!("http://{}:{}/", laddr.ip(), laddr.port());
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
