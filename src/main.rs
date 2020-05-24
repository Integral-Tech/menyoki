#[macro_use]
extern crate log;
mod app;
mod image;
mod record;
mod util;
mod x11;
use self::app::{App, AppSettings};
use self::x11::WindowSystem;
use std::io::Error;

fn main() -> Result<(), Error> {
	let args = util::parse_args();
	util::init_logger().expect("Failed to initialize the logger");

	println!("thank god it's friday");

	let settings = AppSettings::new(args);
	let app = App::new(settings.clone());
	let mut window_system =
		WindowSystem::init(settings).expect("Cannot open display");
	if let Some(record_func) = window_system.get_record_func() {
		let frames = app.record(record_func);
		info!("frames: {}", frames.len());
		if !frames.is_empty() {
			app.save_gif(frames)?;
		} else {
			warn!("No frames found to save.");
		}
	}
	Ok(())
}
