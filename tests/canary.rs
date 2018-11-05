extern crate env_logger;
extern crate sulfur;
#[macro_use]
extern crate lazy_static;


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

#[test]
fn can_navigate() {
    env_logger::try_init().unwrap_or_default();
    let mut s = DRIVER.new_session().expect("new_session");

    let url = "https://en.wikipedia.org/";
    s.visit(url).expect("visit");

    let main_page = s.current_url().expect("current_url");
    assert!(
        main_page.starts_with(url),
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
