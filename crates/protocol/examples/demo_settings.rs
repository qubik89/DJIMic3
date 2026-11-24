//! Prints the DJI Mic 3's setting descriptors as JSON, the same shape the
//! GUI's `snapshot` command sends. Used to build the demo page the website
//! screenshots are taken from (gui/demo.html).
fn main() {
    let model = protocol::model_by_id("dji-mic-3").expect("Mic 3 model");
    println!("{}", serde_json::to_string_pretty(model.settings()).unwrap());
}
