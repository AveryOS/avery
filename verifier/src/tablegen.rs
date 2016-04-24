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
use std::ptr;
use elfloader::*;

use std::sync::atomic::{AtomicBool, Ordering};

#[link(name = "capstone", kind = "static")]
extern {}

#[path = "../capstone/capstone.rs"]
#[allow(dead_code, non_camel_case_types)]
mod capstone;

mod effect;
mod decoder;
mod disasm;
mod table;

use capstone::csh;
use effect::*;

fn capstone_open() -> csh {
	use capstone::*;

	unsafe {
		let mut handle: csh = 0;

		if cs_open(Enum_cs_arch::CS_ARCH_X86, Enum_cs_mode::CS_MODE_64, &mut handle) as u32 != 0 {
			panic!();
		}

		cs_option(handle, Enum_cs_opt_type::CS_OPT_DETAIL, Enum_cs_opt_value::CS_OPT_ON as u64);

		handle
	}
}

fn capstone_close(mut handle: csh) {
	use capstone::*;

	unsafe {
		cs_close(&mut handle);
	}
}

fn capstone(handle: &mut csh, data: &[u8], disp_off: u64, inst: &Inst, effects: &[Effect2]) -> bool {
	use std::ffi::CStr;
	use capstone::*;

	let mut error = false;

	unsafe {
		let mut ci: *mut cs_insn = ptr::null_mut();

		let count = cs_disasm(*handle, data.as_ptr(), data.len() as u64, disp_off, 0, &mut ci);

		if count > 0 {
			let mnemonic = CStr::from_ptr((*ci).mnemonic[..].as_ptr()).to_str().unwrap();
			let ops = CStr::from_ptr((*ci).op_str[..].as_ptr()).to_str().unwrap();
			let desc = format!("{} {}", mnemonic, ops).trim().to_string();

			if inst.desc != desc {
				println!("on {}\n  c: {}\n  m: {}", table::bytes(data), desc, inst.desc);
				error = true;
			}

			if (*ci).size as usize != inst.len {
				println!("on {}\n  len(c): {}\n  len(m): {}", table::bytes(data), (*ci).size, inst.len);
				error = true;
			}

			cs_free(ci, count);
		} else {
			println!("on {}\n  c: invalid\n  m: {}", table::bytes(data), inst.desc);
			error = true;
			//println!("  inst {:?}", inst);
		}
	}

	error
}

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

	let error = &AtomicBool::new(false);

	crossbeam::scope(|scope| {
		let ops = &ops;
		for chunk in seq.chunks(seq.len() / 4 + 1) {
			println!("Spawning thread with {} jobs", chunk.len());
		    scope.spawn(move || {
		    	let mut cp = capstone_open();
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
					let inst = decoder::inst(&mut c, 0, ops);
					//println!("output  {}", inst.desc);
					if capstone(&mut cp, &xs, 0, &inst, effects) {
						error.store(true, Ordering::SeqCst);
					}
					//println!("{} ({}) ..{} => {:?}", table::bytes(&bytes), str, xs.len() - bytes.len(), effects);
				}
				capstone_close(cp);
			});
		}
	});

	if error.load(Ordering::SeqCst) {
		panic!("Output didn't match Capstone");
	}

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
