mod api;
mod server;

// Gui
use eframe::egui;
use egui::Vec2;
use egui_extras::RetainedImage;

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
use std::{thread, time};

pub enum ThreadCom {
    ConnectionActive(bool),
    ClientConnected(Endpoint, ResourceId),
    Screen(Vec<u8>),
    ScreenSize((u32, u32)),
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
    image: Option<RetainedImage>,
    imageSize: Option<Vec2>,
    init_screen_size: Option<Vec2>,
}

impl GuiVars {
    pub fn new() -> Self {
        GuiVars {
            cursor_count: 0,
            image: None,
            imageSize: None,
            init_screen_size: None,
        }
    }
}

// The order of applying changes is from up to down
struct InputOptions {
    // Regulart shifts
    add_to_y: f32,
    add_to_x: f32,
    invert_x: bool,
    invert_y: bool,
    invert_x_with_y: bool,
}

struct MyApp {
    rx_to_gui: Receiver<ThreadCom>,
    network_handler: Arc<NodeHandler<()>>,
    endpoint: Option<Endpoint>,
    gui: GuiVars,
    input_options: InputOptions,
    screen_delay_ms: u32,
}

impl MyApp {
    pub fn send_network(&self, message: FromServerMessage) {
        if let Some(endpoint) = self.endpoint {
            let output_data = bincode::serialize(&message).unwrap();
            let status = self.network_handler.network().send(endpoint, &output_data);
            debug!("Status of message {:?} is {:?}", message, status);
        } else {
            error!("Failed to send network message: missing endpoint");
        }
    }
}

impl Default for MyApp {
    fn default() -> Self {
        // Arguments
        let port = 24356;
        let transport = Transport::Ws;
        let input_options = InputOptions {
            add_to_y: -9.0,
            add_to_x: -8.0,
            invert_x: true,
            invert_y: false,
            invert_x_with_y: true,
        };
        // 1100 uses 30% of cpu
        // 400 uses 100%
        // Using native fbink should help ;p
        let screen_delay_ms = 1100;
        let initial_screen_size = Some((500, 500));

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
            input_options,
            screen_delay_ms,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            //info!("Running events");
            if let Ok(event) = self.rx_to_gui.try_recv() {
                match event {
                    ThreadCom::ConnectionActive(status) => {
                        info!("Gui received connection status: {}", status);
                    }
                    ThreadCom::ClientConnected(endpoint, _resource_id) => {
                        info!("Gui received: ClientConnected");
                        self.endpoint = Some(endpoint);
                        debug!("Creating screen refresh thread");
                        let network_handler_image_delay = self.network_handler.clone();

                        let delay = self.screen_delay_ms as u64;

                        thread::spawn(move || {
                            loop {
                                // TODO: sync, make clicks deliver always, add thread to client for launching fbgrab, sync it too
                                thread::sleep(time::Duration::from_millis(delay));
                                debug!("Refreshing screen");
                                let data =
                                    bincode::serialize(&FromServerMessage::RequestScreen).unwrap();
                                network_handler_image_delay.network().send(endpoint, &data);
                            }
                        });
                    }
                    ThreadCom::Screen(file) => {
                        //debug!("ThreadCom screen called");
                        if let Ok(image) = RetainedImage::from_image_bytes("png", &file) {
                            self.gui.image = Some(image);
                        } else {
                            warn!("Failed to get image from bytes");
                        }
                    }
                    ThreadCom::ScreenSize((x, y)) => {
                        debug!("Setting ui size... x:{}, y:{}", x, y);
                        let vec = Vec2::new(x as f32, y as f32);
                        _frame.set_window_size(vec);
                        ui.set_max_size(vec);
                        ui.set_min_size(vec);
                        self.gui.imageSize = Some(vec);
                    }
                }
            }

            if let Some(pos) = ctx.input(|i| i.pointer.press_origin()) {
                if self.gui.cursor_count == 0 {
                    debug!("Cursor clicked at: {:?}", pos);
                    let mut pos_final = pos;
                    pos_final.y += self.input_options.add_to_y as f32;
                    pos_final.x += self.input_options.add_to_x as f32;

                    // Adjust input
                    if let Some(image_size) = &self.gui.imageSize {
                        // Map the value to the size...
                        let app_size = ui.available_size();
                        debug!("App size: {:?}", app_size);
                        let scale_x = image_size.x / app_size.x;
                        let scale_y = image_size.y / app_size.y;
                        debug!("scale_x:{} scale_y:{}", scale_x, scale_y);
                        pos_final.x *= scale_x;
                        pos_final.y *= scale_y;

                        if self.input_options.invert_x {
                            pos_final.x = image_size.x - pos_final.x;
                        }
                        if self.input_options.invert_y {
                            pos_final.y = image_size.y - pos_final.y;
                        }
                    } else {
                        error!("Failed to adjust input, screen size is missing");
                    }

                    if self.input_options.invert_x_with_y {
                        std::mem::swap(&mut pos_final.x, &mut pos_final.y);
                    }

                    self.send_network(FromServerMessage::Click(
                        pos_final.x as u16,
                        pos_final.y as u16,
                    ));
                }
                self.gui.cursor_count += 1;
                if self.gui.cursor_count >= 3 {
                    self.gui.cursor_count = 0;
                }
            }

            if let Some(image) = &self.gui.image {
                //debug!("Showing image");
                image.show_size(ui, ui.available_size());
            }

            ctx.request_repaint_after(time::Duration::from_millis(self.screen_delay_ms as u64 / 5));
        });
    }
}
