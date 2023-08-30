#![deny(clippy::useless_attribute)]
#![allow(clippy::single_match)]

// Logging
use log::{info, debug};

// Network
use crate::api::{FromClientMessage, FromServerMessage};
use message_io::network::{NetEvent, RemoteAddr, Transport};
use message_io::node::{self, NodeEvent};
use std::time::Duration;

// Device
use crate::device::{click, get_screen, get_screen_size};

pub fn run(transport: Transport, remote_addr: RemoteAddr) {
    let (handler, listener) = node::split();

    let (server_id, local_addr) = handler
        .network()
        .connect(transport, remote_addr.clone())
        .unwrap();

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
                    )
                }
            }
            NetEvent::Accepted(_, _) => unreachable!(), // Only generated when a listener accepts
            NetEvent::Message(_, input_data) => {
                debug!("Received raw input data with length: {}", input_data.len());
                let message: FromServerMessage = bincode::deserialize(input_data).unwrap();
                match message {
                    FromServerMessage::Pong => {
                        info!("Received Pong from server, sending screen size");
                        let message = FromClientMessage::ScreenSize(get_screen_size());
                        let output_data = bincode::serialize(&message).unwrap();
                        handler.network().send(server_id, &output_data);
                    }
                    FromServerMessage::Click(x, y) => {
                        info!("Received Click from server: x:{} y:{}", x, y);
                        click(x, y);
                    }
                    FromServerMessage::RequestScreen => {
                        debug!("Received screen request");
                        let message = FromClientMessage::Screen(get_screen());
                        let output_data = bincode::serialize(&message).unwrap();
                        debug!("Sending raw screen data with length: {}", output_data.len());
                        handler.network().send(server_id, &output_data);
                    }
                }
            }
            NetEvent::Disconnected(_) => {
                info!("Server is disconnected");
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
