#![feature(trace_macros)]
#![feature(log_syntax)]
#![feature(plugin)]
#![feature(const_fn)]
#![feature(slice_patterns)]
//#![cfg_attr(test, feature(plugin, custom_attribute))]
//#![cfg_attr(test, plugin(quickcheck_macros))]

//#[cfg(test)]
//extern crate quickcheck;

extern crate elfloader;
extern crate byteorder;
extern crate core;
extern crate fst;
extern crate crossbeam;

use crossbeam::scope;
use std::fs::File;
use std::io::Read;
use elfloader::*;

#[link(name = "capstone", kind = "static")]
extern {}

#[path = "../capstone/capstone.rs"]
#[allow(dead_code, non_camel_case_types)]
mod capstone;

mod effect;
mod decoder;
mod disasm;
mod table;

fn main() {
	use std::fs::File;
	use std::io;

	use fst::{IntoStreamer, Streamer, Map, MapBuilder};

	// This is where we'll write our map to.
	let mut wtr = io::BufWriter::new(File::create("table.fst").unwrap());

	let mut ops = Vec::new();

	//unsafe { table::DEBUG = true };
	//unsafe { disasm::DEBUG = true };

	table::list_insts(&mut ops, true);

	let mut seq = Vec::new();

	for op in &ops {
		disasm::gen_all(op, &mut seq)
	}

	println!("Testing {} entries...", seq.len());

	crossbeam::scope(|scope| {
		let ops = &ops;
		for chunk in seq.chunks(seq.len() / 4 + 1) {
			println!("Spawning thread with {} jobs", chunk.len());
		    scope.spawn(move || {
		    	let mut cp = decoder::capstone_open();
				for &(ref bytes, ref effects) in chunk {
					let mut xs = bytes.clone();
					for e in effects.iter() {
						for _ in 0..e.trailing_bytes() {
							xs.push(0x1D);
						}
					}
					/*while xs.len() < 16 {
						xs.push(0xBC);
					}*/
					let mut c = decoder::Cursor {
						data: &xs,
						offset: 0,
					};
					//println!("testing {} {:?}", table::bytes(&xs), effects);
					let inst = &decoder::inst(&mut c, 0, ops).0;
					//println!("output  {}", inst.desc);
					decoder::capstone(&mut cp, &xs, 0, &inst, effects);
					//println!("{} ({}) ..{} => {:?}", table::bytes(&bytes), str, xs.len() - bytes.len(), effects);
				}
				decoder::capstone_close(cp);
			});
		}
	});

	println!("Sorting {} entries...", seq.len());

	seq[..].sort_by(|a, b| a.0.cmp(&b.0));


	// Create a builder that can be used to insert new key-value pairs.
	let mut build = MapBuilder::new(wtr).unwrap();

	for op in seq {
		//build.insert(&op.0, op.1 as u64);
	}

	// Finish construction of the map and flush its contents to disk.
	build.finish().unwrap();
}
