extern crate env_logger;
extern crate sulfur;

use sulfur::*;

#[test]
fn can_run_chromedriver() {
    env_logger::try_init().unwrap_or_default();
    let mut driver = ChromeDriver::start().expect("ChromeDriver::start");
    let mut s = driver.new_session().expect("new_session");
    s.close().expect("close");
    driver.close().expect("Close driver");
}

#[test]
fn can_navigate() {
    env_logger::try_init().unwrap_or_default();
    let driver = ChromeDriver::start().expect("ChromeDriver::start");
    let mut s = driver.new_session().expect("new_session");

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
