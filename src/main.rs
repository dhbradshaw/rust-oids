mod app;
mod core;
mod frontend;
mod backend;

#[macro_use]
extern crate log;
extern crate log4rs;
extern crate chrono;
extern crate csv;

#[macro_use]
extern crate bitflags;
extern crate bit_set;
extern crate cgmath;

extern crate wrapped2d;

#[macro_use]
extern crate gfx;
extern crate gfx_device_gl;
extern crate gfx_window_glutin;
extern crate winit;
extern crate glutin;

extern crate dsp;
extern crate portaudio;
extern crate pitch_calc;
extern crate sample;

extern crate rand;
extern crate num;
extern crate itertools;

#[macro_use]
extern crate enum_primitive;
extern crate conrod;

extern crate getopts;
extern crate ctrlc;
#[cfg(unix)]
extern crate thread_priority;

extern crate rustc_serialize as serialize;

fn main() {
	use log4rs::config::*;
	use log4rs::append::console::*;
	use std::env;
	let args = env::args_os().collect::<Vec<_>>();

	let config = Config::builder()
		.appender(Appender::builder().build(
			"stdout".to_string(),
			Box::new(
				ConsoleAppender::builder().build(),
			),
		))
		.logger(Logger::builder().build(
			"gfx_device_gl".to_string(),
			log::LogLevelFilter::Error,
		))
		.logger(Logger::builder().build(
			"rust_oids".to_string(),
			log::LogLevelFilter::Info,
		))
		.build(Root::builder().appender("stdout".to_string()).build(
			log::LogLevelFilter::Info,
		));
	log4rs::init_config(config.unwrap()).unwrap();
	app::run(&args);
}
