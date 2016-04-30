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
use std::ptr;

use std::sync::atomic::{AtomicBool, Ordering};

#[link(name = "capstone", kind = "static")]
extern {}

#[path = "../capstone/capstone.rs"]
#[allow(dead_code, non_snake_case, non_camel_case_types)]
mod capstone;

mod effect;
mod decoder;
mod disasm;
mod table;

use capstone::csh;
use effect::*;
use std::io;
use fst::{IntoStreamer, Streamer, Map, MapBuilder};

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

#[derive(Debug)]
enum Reg {
	IP,
	GP(usize)
}

fn reg(cr: u16) -> Option<Reg> {
	use capstone::x86_reg;
	use capstone::Enum_x86_reg::*;

	match unsafe { ::std::mem::transmute(cr as u32) } {
		X86_REG_AH | X86_REG_AL | X86_REG_AX | X86_REG_EAX | X86_REG_RAX => Some(Reg::GP(0)),
		X86_REG_CH | X86_REG_CL | X86_REG_CX | X86_REG_ECX | X86_REG_RCX => Some(Reg::GP(1)),
		X86_REG_DH | X86_REG_DL | X86_REG_DX | X86_REG_EDX | X86_REG_RDX => Some(Reg::GP(2)),
		X86_REG_BH | X86_REG_BL | X86_REG_BX | X86_REG_EBX | X86_REG_RBX => Some(Reg::GP(3)),
		X86_REG_SPL | X86_REG_SP | X86_REG_ESP | X86_REG_RSP => Some(Reg::GP(4)),
		X86_REG_BPL | X86_REG_BP | X86_REG_EBP | X86_REG_RBP => Some(Reg::GP(5)),
		X86_REG_SIL | X86_REG_SI | X86_REG_ESI | X86_REG_RSI => Some(Reg::GP(6)),
		X86_REG_DIL | X86_REG_DI | X86_REG_EDI | X86_REG_RDI => Some(Reg::GP(7)),
		X86_REG_R8B | X86_REG_R8W | X86_REG_R8D | X86_REG_R8 => Some(Reg::GP(8)),
		X86_REG_R9B | X86_REG_R9W | X86_REG_R9D | X86_REG_R9 => Some(Reg::GP(9)),
		X86_REG_R10B | X86_REG_R10W | X86_REG_R10D | X86_REG_R10 => Some(Reg::GP(10)),
		X86_REG_R11B | X86_REG_R11W | X86_REG_R11D | X86_REG_R11 => Some(Reg::GP(11)),
		X86_REG_R12B | X86_REG_R12W | X86_REG_R12D | X86_REG_R12 => Some(Reg::GP(12)),
		X86_REG_R13B | X86_REG_R13W | X86_REG_R13D | X86_REG_R13 => Some(Reg::GP(13)),
		X86_REG_R14B | X86_REG_R14W | X86_REG_R14D | X86_REG_R14 => Some(Reg::GP(14)),
		X86_REG_R15B | X86_REG_R15W | X86_REG_R15D | X86_REG_R15 => Some(Reg::GP(15)),
		X86_REG_RIP => Some(Reg::IP),
		X86_REG_EFLAGS => None,
		_ => panic!("Unknown register {}", cr),
	}
}

fn capstone(handle: &mut csh, data: &[u8], disp_off: u64, inst: &DecodedInst, effects: &[Effect]) -> bool {
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

			let detail = &mut *(*ci).detail;

			let reads: Vec<_> = detail.regs_read[0..(detail.regs_read_count as usize)].iter().filter_map(|&r| reg(r)).collect();
			let writes: Vec<_> = detail.regs_write[0..(detail.regs_write_count as usize)].iter().filter_map(|&r| reg(r)).collect();

			let x86 = &*detail.x86();

			for op in &x86.operands[0..(x86.op_count as usize)] {

			}

			//println!("on {} {} - reads: {:?} - writes: {:?} - effects {:?}", table::bytes(data), desc, reads, writes, effects);


			if inst.desc != desc {
				println!("on {}\n  c: {}\n  m: {}", table::bytes(data), desc, inst.desc);
				error = true;
			}

			if (*ci).size as usize != inst.len {
				println!("on {}\n  {}\n  len(c): {}\n  len(m): {}", table::bytes(data),  desc, (*ci).size, inst.len);
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
		    	let mut cp = capstone_open();
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

					let inst = disasm::parse(&mut c, 0, format);
					//println!("Instruction {} {} => {:?}", table::bytes(&xs), inst.desc, effects);

					if capstone(&mut cp, &xs, 0, &inst, effects) {
						error.store(true, Ordering::SeqCst);
					}
					//println!("{} ({}) ..{} => {:?}", table::bytes(&bytes), str, xs.len() - bytes.len(), effects);
				}
				capstone_close(cp);
			});
		}
	});

	println!("Building FSM...");

	// Create a builder that can be used to insert new key-value pairs.
	let mut build = MapBuilder::new(wtr).unwrap();

	for op in cases {
		build.insert(&op.0, 0);
	}

	// Finish construction of the map and flush its contents to disk.
	build.finish().unwrap();

	if error.load(Ordering::SeqCst) {
		panic!("Output didn't match Capstone");
	}
}
