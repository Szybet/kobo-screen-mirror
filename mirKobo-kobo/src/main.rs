mod api;
mod client;
mod device;

// Logging
use log::info;

// Network
use message_io::network::{ToRemoteAddr, Transport};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Args {
    #[arg(short, long, help = "Address and port of mirKobo-host using syntax address:port, the default is default InkBox OS usbnet settings", default_value_t = String::from("192.168.2.3:24356"))]
    remote_addr: String,
    #[arg(short, long, help = "Path to fbgrab binary", default_value_t = String::from("/usr/bin/fbgrab"))]
    fbgrab_path: String,
    #[arg(short, long, help = "Path to touch_emulate binary", default_value_t = String::from("./touch_emulate.bin"))]
    touch_emulate_path: String,
    #[arg(short, long, help = "Path to busybox binary (we need fbset for screen size reporting)", default_value_t = String::from("/bin/busybox"))]
    busybox_path: String,
}

fn main() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "debug"),
    );

    // Arguments
    let args = Args::parse();
    //let remote_addr = "192.168.2.3:24356";
    //let remote_addr = "127.0.0.1:24356";

    info!("Starting mirKobo-kobo");
    
    loop {
        let remote_addr = args.remote_addr.to_remote_addr().unwrap();
        client::run(Transport::Ws, remote_addr, &args);
    }
}
