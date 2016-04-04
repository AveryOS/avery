#![feature(core)]
#![feature(trace_macros)]
#![feature(log_syntax)]
#![feature(plugin)]
//#![cfg_attr(test, feature(plugin, custom_attribute))]
//#![cfg_attr(test, plugin(quickcheck_macros))]

//#[cfg(test)]
//extern crate quickcheck;

extern crate elfloader;
extern crate byteorder;
extern crate core;

use std::fs::File;
use std::io::Read;
use elfloader::*;

mod decoder;
mod table;

fn main() {
	let path = "test.elf";
	let mut f = File::open(path).unwrap();
	let mut buffer = Vec::new();
	f.read_to_end(&mut buffer).unwrap();
	let bin = elfloader::Image::new(unsafe { std::mem::transmute(&buffer[..])} ).unwrap();
	/*println!("hi {:?}", bin);
	for h in bin.program_headers() {
		println!("program_header {}", h);
	}*/
	let mut sections = Vec::new();
	for h in bin.sections {
		let mut program = None;

		for p in bin.segments {
			if h.offset == p.offset {
				//println!("matching program header {} EXEC:{}", p, p.flags.0 & elf::PF_X.0 != 0);
				program = Some(p.flags.0 & elf::PF_X.0 != 0);
			}
		}
		sections.push(program.unwrap_or(false));
		//println!("section_header {} {}", bin.section_name(h), h);
	}

	bin.for_each_symbol(|sym, section| {
		let name = bin.symbol_name(sym, section).unwrap();

		if name == "" {
			return;
		}

		match sym.section_index.section() {
			Some(s) => {
				if !bin.section_name(&bin.sections[s]).unwrap().starts_with(".text") {
					return;
				}
			}
			None => (),
		}

		let dump = if bin.header.unwrap().elftype == elf::ET_REL {
			match sym.section_index.section() {
				Some(s) => {
					if sections[s] {
				 		Some((bin.sections[s].data(&bin), sym.value as usize, 0))
					} else {
						None
					}
				}
				None => None,
			}

		} else {
			let p = bin.segments.iter().find(|p| p.vaddr <= sym.value && sym.value + sym.size < p.vaddr + p.filesz && p.flags.executable());
			p.map(|p| (p.data(&bin), (sym.value - p.vaddr) as usize, p.vaddr))
		};

		if let Some((data, offset, disp_off)) = dump {
			println!("dumping symbol {} {:x} {}", name, offset, sym);
			if sym.size != 0 {
				decoder::decode(data, offset, sym.size as usize, disp_off);
			}
		}
	});

	println!("Done!");
}
