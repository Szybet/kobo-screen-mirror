// Logging
use log::{debug, info, warn};

// Network
use crate::api::{FromClientMessage, FromServerMessage};
use message_io::network::{NetEvent, Transport};
use message_io::node::{self, NodeHandler, NodeListener};
use std::net::SocketAddr;

// Threads
use std::sync::mpsc::Sender;
use std::thread;
use crate::ThreadCom;
use std::sync::Arc;

pub fn run(handler: Arc<NodeHandler<()>>, listener: NodeListener<()>, tx_to_gui: Sender<ThreadCom>) {


    listener.for_each(move |event| match event.network() {
        NetEvent::Connected(_, _) => (),
        NetEvent::Accepted(endpoint, _listener_id) => {
            // Only connection oriented protocols will generate this event
            info!("Client ({}) connected", endpoint.addr());
            tx_to_gui.send(ThreadCom::ClientConnected(endpoint, _listener_id)).unwrap();
        }
        NetEvent::Message(endpoint, input_data) => {
            debug!("Received raw input data with length: {}", input_data.len());
            let message: FromClientMessage = bincode::deserialize(input_data).unwrap();
            match message {
                FromClientMessage::Ping => {
                    info!("Received Ping from client");
                    tx_to_gui.send(ThreadCom::ConnectionActive(true)).unwrap();
                    let output_data = bincode::serialize(&FromServerMessage::Pong).unwrap();
                    info!("Sending Pong");
                    handler.network().send(endpoint, &output_data);
                }
                FromClientMessage::Screen(file) => {
                    debug!("Received Screen from client");
                    tx_to_gui.send(ThreadCom::Screen(file)).unwrap();
                }
                FromClientMessage::ScreenSize((x, y)) => {
                    debug!("Received Screen size from client");
                    tx_to_gui.send(ThreadCom::ScreenSize((x, y))).unwrap();
                }
            }
        }
        NetEvent::Disconnected(endpoint) => {
            info!("Client ({}) disconnected", endpoint.addr(),);
        }
    });
}
