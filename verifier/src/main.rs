#![feature(core)]
#![feature(trace_macros)]
#![feature(log_syntax)]

extern crate elfloader;
extern crate byteorder;
extern crate core;

use std::fs::File;
use std::io::Read;
use elfloader::*;

mod decoder;
mod table;

fn main() {
	let path = "kernel.elf";
	let mut f = File::open(path).unwrap();
	let mut buffer = Vec::new();
	f.read_to_end(&mut buffer).unwrap();
	let bin = elfloader::ElfBinary::new(path, unsafe { std::mem::transmute(&buffer[..])} ).unwrap();
	/*println!("hi {:?}", bin);
	for h in bin.program_headers() {
		println!("program_header {}", h);
	}*/
	let mut sections = Vec::new();
	for h in bin.section_headers() {
		let mut program = None;

		for p in bin.program_headers() {
			if h.offset == p.offset {
				//println!("matching program header {} EXEC:{}", p, p.flags.0 & elf::PF_X.0 != 0);
				program = Some(p.flags.0 & elf::PF_X.0 != 0);
			}
		}
		sections.push(program.unwrap_or(false));
		//println!("section_header {} {}", bin.section_name(h), h);
	}

	bin.for_each_symbol(|sym| {
		let name = bin.symbol_name(sym);

		if name == "" {
			return;
		}

		match sym.section_index.section() {
			Some(s) => {
				if !bin.section_name(&bin.section_headers()[s]).starts_with(".text") {
					return;
				}
			}
			None => (),
		}

		let dump = if bin.header.elftype == elf::ET_REL {
			match sym.section_index.section() {
				Some(s) => {
					if sections[s] {
				 		Some((bin.section_data(&bin.section_headers()[s]), sym.value as usize))
					} else {
						None
					}
				}
				None => None,
			}

		} else {
			let p = bin.program_headers().iter().find(|p| p.vaddr <= sym.value && sym.value + sym.size < p.vaddr + p.filesz && p.flags.executable());
			p.map(|p| (bin.program_data(p), (sym.value - p.vaddr) as usize))
		};

		if let Some((data, offset)) = dump {
			println!("dumping symbol {} {:x} {}", name, offset, sym);
			decoder::decode(data, offset, sym.size as usize);
		}
	});
}
