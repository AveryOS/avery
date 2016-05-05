#![feature(trace_macros)]
#![feature(log_syntax)]
#![feature(plugin)]
#![feature(const_fn)]
#![feature(inclusive_range_syntax)]
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

use std::sync::atomic::{AtomicBool, Ordering};

mod effect;
mod decoder;
mod disasm;
mod table;

use effect::*;
use std::io;
use fst::{IntoStreamer, Streamer, Map, MapBuilder};

fn main() {
	// This is where we'll write our map to.
	let wtr = io::BufWriter::new(File::create("table.fst").unwrap());

	let mut ops = Vec::new();

	//unsafe { table::DEBUG = true };
	//unsafe { disasm::DEBUG = true };

	table::list_insts(&mut ops, true);

	let mut cases = Vec::new();

	for op in &ops {
		//println!("Generating {:?}", op);
		disasm::gen_all(op, &mut cases)
	}

	println!("Sorting {} entries...", cases.len());

	cases[..].sort_by(|a, b| a.0.cmp(&b.0));

	let cases = &cases;

	println!("Testing {} entries...", cases.len());

	let error = &AtomicBool::new(false);

	crossbeam::scope(|scope| {
		let ops = &ops;
		for chunk in cases.chunks(cases.len() / 4 + 1) {
			println!("Spawning thread with {} jobs", chunk.len());
		    scope.spawn(move || {
		    	let mut cp = decoder::capstone_open();
				for &(ref bytes, ref effects, ref format) in chunk {
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
						offset: bytes.len(),
					};

					println!("Pre-Instruction {} => {:?}", table::bytes(&xs), effects);
					let inst = disasm::parse(&mut c, 0, format);
					println!("Instruction {} {} => {:?}", table::bytes(&xs), inst.desc, effects);
					effect::Effect::encode(effects);

					if decoder::capstone(&mut cp, &xs, 0, &inst, effects) {
						error.store(true, Ordering::SeqCst);
					}
					//println!("{} ({}) ..{} => {:?}", table::bytes(&bytes), str, xs.len() - bytes.len(), effects);
				}
				decoder::capstone_close(cp);
			});
		}
	});

	println!("Building FSM...");

	// Create a builder that can be used to insert new key-value pairs.
	let mut build = MapBuilder::new(wtr).unwrap();

	for op in cases {
		build.insert(&op.0, effect::Effect::encode(&op.1) as u64).unwrap();
	}

	// Finish construction of the map and flush its contents to disk.
	build.finish().unwrap();

	if error.load(Ordering::SeqCst) {
		panic!("Output didn't match Capstone");
	}
}
