#![feature(core)]
#![feature(trace_macros)]
#![feature(log_syntax)]
#![feature(plugin)]
//#![cfg_attr(test, feature(plugin, custom_attribute))]
//#![cfg_attr(test, plugin(quickcheck_macros))]

//#[cfg(test)]
//extern crate quickcheck;

extern crate rustc_serialize;
extern crate elfloader;
extern crate byteorder;
extern crate core;

use rustc_serialize::hex::FromHex;
use std::sync::Mutex;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read;
use elfloader::*;

mod decoder;
mod table;

fn main() {
	let file_rem = Mutex::new(OpenOptions::new().write(true).append(true).open("rem-errors").unwrap());
	let mut file = File::open("errors").unwrap();
	let mut s = String::new();
	file.read_to_string(&mut s);
	for line in s.lines() {
		let mut xs = line.from_hex().unwrap();
		decoder::decode_test_allp(&xs, &file_rem);
	}

}
