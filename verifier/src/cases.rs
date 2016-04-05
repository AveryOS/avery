#![feature(trace_macros)]
#![feature(log_syntax)]
#![feature(plugin)]
#![feature(const_fn)]
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
	let file_rem = Mutex::new(OpenOptions::new().create(true).write(true).truncate(true).open("rem-errors").unwrap());
	let mut file = File::open("errors").unwrap();
	let mut s = String::new();
	unsafe { table::DEBUG = true };
	file.read_to_string(&mut s);
	let mut lines: Vec<&str> = s.lines().collect();
	lines.sort();
	lines.dedup();
	for line in lines.iter() {
		let mut xs = line.from_hex().unwrap();
		while xs.len() < 16 {
			xs.push(0);
		}
		decoder::decode_test(&xs, &file_rem);
	}

}
