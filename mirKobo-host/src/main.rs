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
use message_io::network::{Endpoint, ResourceId, Transport};
use message_io::node::{self, NodeHandler};
use std::net::ToSocketAddrs;

// Threads
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::{thread, time};

// Arguments
use clap::Parser;

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
    image_size: Option<Vec2>,
}

impl GuiVars {
    pub fn new() -> Self {
        GuiVars {
            cursor_count: 0,
            image: None,
            image_size: None,
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
    reverse_coordinates: bool,
}

struct MyApp {
    rx_to_gui: Receiver<ThreadCom>,
    network_handler: Arc<NodeHandler<()>>,
    endpoint: Option<Endpoint>,
    gui: GuiVars,
    input_options: InputOptions,
    screen_delay_ms: u32,
    initial_screen_size: Option<(u32, u32)>,
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

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Args {
    #[arg(short, long, help = "Network port to use", default_value_t = 24356)]
    port: u16,
    #[arg(short, long, help = "Shift x in pixels (compensate for window frame for example), in touch", default_value_t = -8.0)]
    add_to_x: f32,
    #[arg(short, long, help = "Shift y in pixels (compensate for window frame for example), in touch", default_value_t = -9.0)]
    add_to_y: f32,
    // Why default messages aren't shown?
    #[arg(short, long, help = "Invert x, in touch [default: true]", default_value_t = true)]
    invert_x: bool,
    #[arg(short, long, help = "Invert y, in touch [default: false]", default_value_t = false)]
    invert_y: bool,
    #[arg(short, long, help = "Make x y and y x, in touch [default: true]", default_value_t = true)]
    reverse_coordinates: bool,
    #[arg(short, long, help = "Delay between screen refreshes in ms, 400 is fast but takes too much cpu, 1100 is good enough - if you want to go as fast as possible, look for \'Request for screen ignored, it's already in make\' errors on the client - it indicates that grabbing the framebuffer / network speed is the bottleneck", default_value_t = 1100)]
    screen_delay_ms: u32,
    #[arg(short, long, help = "Launch the app with width, initial app width 0 makes the app of the size of the ereader framebuffer", default_value_t = 450)]
    initial_screen_x: u32,
    #[arg(short, long, help = "Launch the app with height, initial app height 0 makes the app of the size of the ereader framebuffer", default_value_t = 600)]
    initial_screen_y: u32,
}

impl Default for MyApp {
    fn default() -> Self {
        // Arguments
        let args = Args::parse();

        let port = args.port;
        let input_options = InputOptions {
            add_to_y: args.add_to_y,
            add_to_x: args.add_to_x,
            invert_x: args.invert_x,
            invert_y: args.invert_y,
            reverse_coordinates: args.reverse_coordinates,
        };
        // 1100 uses 30% of cpu
        // 400 uses 100%
        // Using native fbink should help ;p
        let screen_delay_ms = args.screen_delay_ms;
        let mut initial_screen_size = None;
        if args.initial_screen_x != 0 && args.initial_screen_y != 0 {
            initial_screen_size = Some((args.initial_screen_x, args.initial_screen_y));
        }

        // Threads
        let (tx_to_gui, rx_to_gui) = mpsc::channel();

        // Network
        let addr = ("0.0.0.0", port).to_socket_addrs().unwrap().next().unwrap();
        let (handler, listener) = node::split::<()>();
        let network_handler = Arc::new(handler);
        let transport = Transport::Ws;
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
            initial_screen_size,
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
                        if let Some(size) = self.initial_screen_size {
                            _frame.set_window_size(Vec2::new(size.0 as f32, size.1 as f32));
                        } else {
                            _frame.set_window_size(vec);
                        }
                        ui.set_max_size(vec);
                        ui.set_min_size(vec);
                        self.gui.image_size = Some(vec);
                    }
                }
            }

            if let Some(pos) = ctx.input(|i| i.pointer.press_origin()) {
                if self.gui.cursor_count == 0 {
                    debug!("Cursor clicked at: {:?}", pos);
                    let mut pos_final = pos;
                    pos_final.y += self.input_options.add_to_y;
                    pos_final.x += self.input_options.add_to_x;

                    // Adjust input
                    if let Some(image_size) = &self.gui.image_size {
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

                    if self.input_options.reverse_coordinates {
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
