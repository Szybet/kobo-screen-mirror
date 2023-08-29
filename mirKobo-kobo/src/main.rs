mod api;
mod client;
mod device;

// Logging
use log::{debug, info, warn};

// Network
use message_io::network::{ToRemoteAddr, Transport};

fn main() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "debug"),
    );
    debug!("Starting mirKobo-kobo");

    // Arguments
    let remote_addr = "192.168.2.3:24356";
    //let remote_addr = "127.0.0.1:24356";

    let remote_addr = remote_addr.to_remote_addr().unwrap();
    client::run(Transport::Tcp, remote_addr);
}
