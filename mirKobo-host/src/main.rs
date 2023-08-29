mod api;
mod server;

// Gui
use eframe::egui;
use egui::Pos2;

// Logging
use log::{debug, error, info, warn};

// Network
pub use api::{FromClientMessage, FromServerMessage};
use message_io::network::NetEvent;
use message_io::network::{Endpoint, ResourceId, Transport};
use message_io::node::{self, NodeHandler};
use std::net::SocketAddr;
use std::net::ToSocketAddrs;

// Threads
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;

pub enum ThreadCom {
    ConnectionActive(bool),
    ClientConnected(Endpoint, ResourceId),
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init_from_env(env_logger::Env::default().filter_or(
        env_logger::DEFAULT_FILTER_ENV,
        "debug,eframe=info,egui=info,winit=info",
    ));
    debug!("Starting mirKobo-host");

    // Gui
    let options = eframe::NativeOptions {
        ..Default::default()
    };
    eframe::run_native("mirKobo", options, Box::new(|_cc| Box::<MyApp>::default()))
}

struct GuiVars {
    cursor_count: i32, // For some reason it reports 3 events, so let's ignore them
}

impl GuiVars {
    pub fn new() -> Self {
        GuiVars {
            cursor_count: 0,
        }
    }
}

struct MyApp {
    rx_to_gui: Receiver<ThreadCom>,
    network_handler: Arc<NodeHandler<()>>,
    endpoint: Option<Endpoint>,
    gui: GuiVars,
}

impl MyApp {
    pub fn send_network(&self, message: FromServerMessage) {
        if let Some(endpoint) = self.endpoint {
            let output_data = bincode::serialize(&message).unwrap();
            self.network_handler.network().send(endpoint, &output_data);
        } else {
            error!("Failed to send network message: missing endpoint");
        }
    }
}

impl Default for MyApp {
    fn default() -> Self {
        // Arguments
        let port = 24356;
        let transport = Transport::Tcp;

        // Threads
        let (tx_to_gui, rx_to_gui) = mpsc::channel();

        // Network
        let addr = ("0.0.0.0", port).to_socket_addrs().unwrap().next().unwrap();
        let (handler, listener) = node::split::<()>();
        let network_handler = Arc::new(handler);
        match network_handler.network().listen(transport, addr) {
            Ok((_id, real_addr)) => info!("Server running at {} by {}", real_addr, transport),
            Err(_) => error!("Can not listening at {} by {}", addr, transport),
        }

        let network_handler_server = network_handler.clone();
        thread::spawn(move || {
            tx_to_gui.send(ThreadCom::ConnectionActive(false)).unwrap();
            server::run(network_handler_server, listener, tx_to_gui); // Enable websockets
        });
        Self {
            rx_to_gui,
            network_handler: network_handler.clone(),
            endpoint: None,
            gui: GuiVars::new(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Ok(event) = self.rx_to_gui.try_recv() {
                match event {
                    ThreadCom::ConnectionActive(status) => {
                        info!("Gui received connection status: {}", status);
                    }
                    ThreadCom::ClientConnected(endpoint, _resource_id) => {
                        info!("Gui received: ClientConnected");
                        self.endpoint = Some(endpoint);
                    }
                }
            }

            if let Some(pos) = ctx.input(|i| i.pointer.press_origin()) {
                self.gui.cursor_count += 1;
                if self.gui.cursor_count == 3 {
                    self.gui.cursor_count = 0;
                    debug!("Cursor clicked at: {:?}", pos);
                    self.send_network(FromServerMessage::Click(pos.x as u16, pos.y as u16)); // test
                }
            }
        });
    }
}
