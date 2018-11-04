extern crate sulfur;

use sulfur::*;

const WD_HUB: &'static str = "http://localhost:4444/wd/hub/";

#[test]
fn can_create_new_session() {
    let s = Client::new(WD_HUB, NewSessionReq::chrome()).expect("session::new chrome");

    println!("Sess: {:#?}", s);

    s.close().expect("close")
}
