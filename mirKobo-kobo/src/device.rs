// Logging
use log::{debug, info, warn};
use std::process::Command;
pub fn click(x: u16, y: u16) {
    debug!("Launching click");

    let res = Command::new("/touch_emulate.bin")
    .arg("touch")
    .arg("/dev/input/event1")
    .arg(x.to_string())
    .arg(y.to_string())
    .output()
    .expect("failed to execute process");

    debug!("Command output: {:?}", res.stdout);
}
