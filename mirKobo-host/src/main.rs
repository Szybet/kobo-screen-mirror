use eframe::egui;
use log::{debug, error, log_enabled, info, Level};

fn main() -> Result<(), eframe::Error> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "debug"),
    );
    let options = eframe::NativeOptions {
        ..Default::default()
    };
    eframe::run_native(
        "mirKobo",
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}

struct MyApp {
    
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(pos) = ctx.input(|i| i.pointer.press_origin()) {
                debug!("Cursor: {:?}",pos);
            }
            
        });
    }
}
