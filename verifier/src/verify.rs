#![feature(trace_macros)]
#![feature(log_syntax)]
#![feature(plugin)]
#![feature(const_fn)]
#![feature(slice_patterns)]
#![feature(inclusive_range_syntax)]
#![allow(dead_code)]
//#![cfg_attr(test, feature(plugin, custom_attribute))]
//#![cfg_attr(test, plugin(quickcheck_macros))]

//#[cfg(test)]
//extern crate quickcheck;

extern crate elfloader;
extern crate byteorder;
extern crate core;
extern crate getopts;

use getopts::Options;
use std::fs::File;
use std::path::Path;
use std::io::Read;
use elfloader::*;

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

	bin.for_each_symbol(|sym, section| {
		let name = bin.symbol_name(sym, section).unwrap();

		if name == "" {
			return;
		}

		let dump = if bin.header.unwrap().elftype == elf::ET_REL {
			match sym.section_index.section() {
				Some(s) => {
					let section = &bin.sections[s];
					if section.shtype == elf::SHT_PROGBITS && (section.flags.0 & elf::SHF_EXECINSTR.0 != 0)  {
				 		Some((bin.sections[s].data(&bin), sym.value as usize, 0))
					} else {
						None
					}
				}
				None => None
			}
		} else {
			let p = bin.segments.iter().find(|p| p.vaddr <= sym.value && sym.value + sym.size < p.vaddr + p.filesz && p.flags.executable());
			p.map(|p| (p.data(&bin), (sym.value - p.vaddr) as usize, p.vaddr))
		};

		if let Some((data, offset, disp_off)) = dump {
			println!("dumping symbol {} {:x} {}", name, offset, sym);
			if sym.size != 0 {
				x86_decoder::decode(data, offset, sym.size as usize, disp_off);
			}
		}
	});

	println!("Done!");
}
