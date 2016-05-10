#![feature(trace_macros)]
#![feature(log_syntax)]
#![feature(plugin)]
#![feature(const_fn)]
#![feature(slice_patterns)]
#![feature(stmt_expr_attributes)]
#![feature(inclusive_range_syntax)]
#![feature(question_mark)]
#![feature(nonzero)]
#![allow(dead_code)]
//#![cfg_attr(test, feature(plugin, custom_attribute))]
//#![cfg_attr(test, plugin(quickcheck_macros))]

//#[cfg(test)]
//extern crate quickcheck;

extern crate elfloader;
extern crate byteorder;
extern crate core;
extern crate getopts;
extern crate time;

use getopts::Options;
use std::fs::File;
use std::io::Read;
use elfloader::*;
use time::PreciseTime;

mod effect;
mod decoder;
mod table;
mod disasm;
mod x86_opcodes;
mod x86_decoder;

fn main() {
	let args: Vec<String> = std::env::args().collect();

	let mut opts = Options::new();
	opts.optflag("b", "brief", "print a brief error on stderr");
	opts.reqopt("f", "file", "the file to verify", "<file>");
	let matches = match opts.parse(&args[1..]) {
		Ok(m) => m,
		Err(f) => {
			println!("{}", f.to_string());
			return
		}
	};

	let path = matches.opt_str("f").unwrap();

	std::env::args().nth(1).unwrap();
	println!("Dumping {}", path);
	let mut f = File::open(path).unwrap();
	let mut buffer = Vec::new();
	f.read_to_end(&mut buffer).unwrap();
	let bin = elfloader::Image::new(unsafe { std::mem::transmute(&buffer[..])} ).unwrap();

	for section in bin.sections{
		println!("Section {:?}", section);
	}


	let start = PreciseTime::now();

	bin.for_each_symbol(|sym, section| {
		if sym.sym_type() != elf::STT_FUNC {
			return;
		}

		let name = bin.symbol_name(sym, section).unwrap();

		let dump = match sym.section_index.section() {
			Some(s) => {
				let section = &bin.sections[s];
				if section.shtype == elf::SHT_PROGBITS && (section.flags.0 & elf::SHF_EXECINSTR.0 != 0)  {
					Some(if bin.header.unwrap().elftype == elf::ET_REL {
						(bin.sections[s].data(&bin), sym.value, 0)
					} else {
						(bin.sections[s].data(&bin), sym.value - section.addr, section.addr)
					})
			 		
				} else {
					None
				}
			}
			None => None
		};

		if let Some((data, offset, disp_off)) = dump {
			if x86_decoder::DEBUG {
				println!("dumping symbol {} {:x} {}", name, offset, sym);
			}
			let data = &data[(offset as usize)..(offset as usize + sym.size as usize)];
			x86_decoder::decode(data, disp_off + offset).unwrap()
		}
	});

	let time = start.to(PreciseTime::now());

	let insts = unsafe { x86_decoder::INSTRUCTIONS };

	let tpi = time.num_nanoseconds().map(|n| n as f64 / insts as f64);

	println!("Done! {} instruction(s) in {}, {:?} ns / instruction", insts, time, tpi);
}
