extern crate sulfur;

use sulfur::*;

const WD_HUB: &'static str = "http://localhost:4444/wd/hub/";

#[test]
fn opens_and_closes() {
    let client = Driver::new(WD_HUB, chrome()).expect("new driver");

    client.close().expect("close");
}
