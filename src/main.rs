mod window_manager;
mod render_manager;
mod font_loader;
mod utils;

use std::{env::Args, ops::Deref};

use font_loader::{FontFile, TableDirectory};
use winit::event_loop::{ControlFlow, EventLoop};
use window_manager::App;

fn main() {
    let args: Args = std::env::args();
    let mut args = args.skip(1);
    let file_path = args.next().unwrap();
    let font_file = FontFile::new(&file_path);
    
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App {
        font_file: Some(font_file),
        ..Default::default()
    };
    if let Err(e) = event_loop.run_app(&mut app) {
        eprintln!("Error: {:?}", e);
    }
}