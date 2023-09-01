// Logging
use log::debug;
use std::process::Command;
pub fn click(x: u16, y: u16, bin_path: &str) {
    debug!("Launching click");

    Command::new(bin_path)
        .arg("touch")
        .arg("/dev/input/event1")
        .arg(x.to_string())
        .arg(y.to_string())
        .status()
        .expect("failed to execute process");

    //debug!("Command output: {:?}", res.stdout);
}

pub fn get_screen(bin_path: &str) -> Vec<u8> {
    // /usr/bin/fbgrab -a -z 9 /tmp/mirror.png
    debug!("Launching fbgrab");
    Command::new(bin_path)
        .arg("-a")
        .arg("-z")
        .arg("0")
        .arg("/tmp/mirror.png")
        .status()
        .expect("failed to execute process");

    //debug!("Command output: {:?}", res.stdout);
    debug!("fbgrab finished");

    std::fs::read("/tmp/mirror.png").unwrap()
}

pub fn get_screen_size(bin_path: &str) -> (u32, u32) {
    // /bin/busybox fbset
    let res = Command::new(bin_path)
        .arg("fbset")
        .output()
        .expect("failed to execute process");

    let output = String::from_utf8(res.stdout).unwrap();
    let lines: Vec<&str> = output.split('\n').collect();
    let line: Vec<&str> = lines[3].split(' ').collect();
    debug!("line: {:?}", line);
    let x: u32 = line[1].parse().unwrap();
    let y: u32 = line[2].parse().unwrap();

    debug!("Output of fbset: {}", output);
    (x, y)
}
