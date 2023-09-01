#![deny(clippy::useless_attribute)]
#![allow(clippy::single_match)]

// Logging
use log::{debug, error, info};

// Network
use crate::api::{FromClientMessage, FromServerMessage};
use message_io::network::{NetEvent, RemoteAddr, Transport};
use message_io::node::{self, NodeEvent};

// Device
use crate::device::{click, get_screen, get_screen_size};

// Other
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;
use crate::Args;

// We don't allow to loose any of those events
enum ImportantJobs {
    SendClick(u16, u16),
    Stop,
}

// We allow to loose those events
enum LooseJobs {
    SendScreen,
    Stop,
}

pub fn run(transport: Transport, remote_addr: RemoteAddr, args: &Args) {
    let (handler_regular, listener) = node::split();
    let handler = Arc::new(handler_regular);

    let (server_id, local_addr) = handler
        .network()
        .connect(transport, remote_addr.clone())
        .unwrap();

    let (tx_to_imp, rx_to_imp) = mpsc::channel(); // We want not synced because we don't want to loose any input
    let touch_emulate_path = args.touch_emulate_path.clone();
    thread::spawn(move || loop {
        if let Ok(event) = rx_to_imp.recv() {
            match event {
                ImportantJobs::SendClick(x, y) => {
                    info!("Received Click from server: x:{} y:{}", x, y);
                    click(x, y, &touch_emulate_path);
                }
                ImportantJobs::Stop => {
                    break;
                }
            }
        }
    });

    let (tx_to_loose, rx_to_loose) = mpsc::sync_channel(1); // We want synced channel because of try_send
    let handler_thread = handler.clone();
    let fbgrab_path = args.fbgrab_path.clone();
    thread::spawn(move || loop {
        if let Ok(event) = rx_to_loose.recv() {
            match event {
                LooseJobs::SendScreen => {
                    let message = FromClientMessage::Screen(get_screen(&fbgrab_path));
                    let output_data = bincode::serialize(&message).unwrap();
                    debug!("Sending raw screen data with length: {}", output_data.len());
                    handler_thread.network().send(server_id, &output_data);
                }
                LooseJobs::Stop => {
                    break;
                }
            }
        }
    });

    listener.for_each(move |event| match event {
        NodeEvent::Network(net_event) => match net_event {
            NetEvent::Connected(_, established) => {
                if established {
                    info!(
                        "Connected to server at {} by {}",
                        server_id.addr(),
                        transport
                    );
                    info!("Client identified by local port: {}", local_addr.port());
                    handler.signals().send(FromClientMessage::Ping);
                } else {
                    info!(
                        "Cannot connect to server at {} by {}",
                        remote_addr, transport
                    );
                    info!("Retrying in 3 seconds...");
                    tx_to_loose.send(LooseJobs::Stop).unwrap();
                    tx_to_imp.send(ImportantJobs::Stop).unwrap();
                    thread::sleep(Duration::from_secs(3));
                    handler.stop();
                }
            }
            NetEvent::Accepted(_, _) => unreachable!(), // Only generated when a listener accepts
            NetEvent::Message(_, input_data) => {
                debug!("Received raw input data with length: {}", input_data.len());
                let message: FromServerMessage = bincode::deserialize(input_data).unwrap();
                match message {
                    FromServerMessage::Pong => {
                        info!("Received Pong from server, sending screen size");
                        let message = FromClientMessage::ScreenSize(get_screen_size(&args.busybox_path));
                        let output_data = bincode::serialize(&message).unwrap();
                        handler.network().send(server_id, &output_data);
                    }
                    FromServerMessage::Click(x, y) => {
                        tx_to_imp.send(ImportantJobs::SendClick(x, y)).unwrap();
                    }
                    FromServerMessage::RequestScreen => {
                        debug!("Received screen request");
                        // Avoid launching many threads...
                        if tx_to_loose.try_send(LooseJobs::SendScreen).is_err() {
                            error!("Request for screen ignored, it's already in make");
                        }
                    }
                }
            }
            NetEvent::Disconnected(_) => {
                info!("Server is disconnected");
                info!("Retrying in 3 seconds...");
                tx_to_loose.send(LooseJobs::Stop).unwrap();
                tx_to_imp.send(ImportantJobs::Stop).unwrap();
                thread::sleep(Duration::from_secs(3));
                handler.stop();
            }
        },
        NodeEvent::Signal(signal) => match signal {
            FromClientMessage::Ping => {
                info!("Sending Ping");
                let message = FromClientMessage::Ping;
                let output_data = bincode::serialize(&message).unwrap();
                handler.network().send(server_id, &output_data);
                //handler.signals().send_with_timer(Signal::Greet, Duration::from_secs(1));
            }
            _ => {}
        },
    });
}
