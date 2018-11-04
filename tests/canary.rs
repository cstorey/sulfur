extern crate sulfur;

use sulfur::*;

const WD_HUB: &'static str = "http://localhost:4444/wd/hub/";

#[test]
fn can_create_new_session() {
    let s = Client::new(WD_HUB, NewSessionReq::chrome()).expect("session::new chrome");

    println!("Sess: {:#?}", s);

    s.close().expect("close")
}

#[test]
fn can_navigate() {
    let s = Client::new(WD_HUB, NewSessionReq::chrome()).expect("session::new chrome");

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
