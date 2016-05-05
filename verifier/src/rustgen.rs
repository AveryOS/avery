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

use std::process::Command;
use std::path::Path;
use std::fs::File;
use std::io::Write;
use std::collections::HashMap;

mod effect;
mod decoder;
mod table;
mod disasm;

use effect::{Size, Operand, Access, Regs};

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
struct Operands {
	pub operands: Vec<(Operand, Size, Access)>,
	pub mov: bool,
	pub lea: bool,
	pub accesses: Vec<(usize, Access)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Inst(String, usize);

impl Inst {
	fn note(&self, writer: &mut Write) {
		write!(writer, " /* {} */ ", self.0).unwrap();
	}
	fn write(&self, writer: &mut Write) {
		write!(writer, "{:#02x}", self.1).unwrap();
		self.note(writer);
	}
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ByteTrie {
	Empty,
	Decoded(Multiplexer),
	Map(HashMap<u8, Box<ByteTrie>>),
}

impl ByteTrie {
	fn get_new(&mut self, bytes: &[u8], mut taken: Vec<u8>) -> &mut ByteTrie {
		match bytes.split_first() {
			Some((&b, rest)) => {
				match self {
					&mut ByteTrie::Empty => {
						*self = ByteTrie::Map(HashMap::new());
						self.get_new(bytes, taken)
					},
					&mut ByteTrie::Map(ref mut map) => {
						taken.push(b);
						map.entry(b).or_insert(Box::new(ByteTrie::Empty)).get_new(rest, taken)
					}
					_ => panic!("Opcode prefix taken! {}", table::bytes(&taken)),
				}
			}
			None => {
				match self {
					&mut ByteTrie::Empty | &mut ByteTrie::Decoded(..) => self,
					_ => panic!("Opcode taken! {}", table::bytes(&taken)),
				}
			}
		}
	}

	fn write(&self, writer: &mut Write) {
		match *self {
			ByteTrie::Empty => panic!(),
			ByteTrie::Decoded(ref multi) => multi.write(writer),
			ByteTrie::Map(ref map) => {
				writer.write_all("let byte = if let Some((&byte, rest)) = bytes.split_first() { bytes = rest; b } else { return None };".as_bytes()).unwrap();
				writer.write_all("match byte { ".as_bytes()).unwrap();
				for (b, tree) in map {
					write!(writer, "{:#02x} => {{ ", b).unwrap();
					tree.write(writer);
					writer.write_all("}".as_bytes()).unwrap();
				}
				writer.write_all("_ => return None }".as_bytes()).unwrap();
			}
		}
	}
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Multiplexer {
	Opcode(HashMap<u8, Inst>),
	Prefix(HashMap<Vec<u8>, Inst>),
}

impl Multiplexer {
	fn write(&self, writer: &mut Write) {
		match *self {
			Multiplexer::Opcode(ref map) => {
				writer.write_all("let opcode = if let Some(modrm) = bytes.first() { (modrm >> 3) & 7 } else { return None };".as_bytes()).unwrap();
				writer.write_all("match opcode { ".as_bytes()).unwrap();
				for (b, i) in map {
					write!(writer, "{:#02x} => {{", b).unwrap();
					i.write(writer);
					writer.write_all("}".as_bytes()).unwrap();
				}
				writer.write_all("_ => return None }".as_bytes()).unwrap();
			}
			Multiplexer::Prefix(ref map) => {
				if map.len() == 1 && map.get(&[][..]).is_some() {
					(map.iter().next().unwrap().1).write(writer);
				} else {
					for (b, i) in map {
						if !b.is_empty() {
							write!(writer, "if ").unwrap();
							for (i, p) in b.iter().enumerate() {
								write!(writer, "prefix({:#02x})", p).unwrap();
								if i != b.len() - 1 {
									writer.write_all("&&".as_bytes()).unwrap();
								}
							}
							writer.write_all("{ return ".as_bytes()).unwrap();
							i.write(writer);
							writer.write_all("}".as_bytes()).unwrap();
						}
					}
					if let Some(i) = map.get(&[][..]) {
						i.write(writer);
					} else {
						writer.write_all("return None".as_bytes()).unwrap();
					}
				}
			}
		}
	}
}

fn main() {
	let mut ops = Vec::new();

	unsafe { table::DEBUG = true };

	table::list_insts(&mut ops, false);
	
	let mut operands = Vec::new();

	let map_ops = |op: &effect::Inst| -> Operands {
		let ops = op.operands.iter().map(|o| match *o {
			(Operand::RmOpcode(..), s, a) => (Operand::Rm(Regs::GP), s, a),
			ref e => e.clone()
		}).collect();

		Operands {
			mov: op.name == "mov" && (&op.bytes == &[0x89] || &op.bytes == &[0x8b]),
			lea: op.name == "lea",
			accesses: op.accesses.clone(),
			operands: ops,
		}
	};

	for op in &ops {
		operands.push(map_ops(&op));
	}

	operands.sort();
	operands.dedup();

	println!("Formats {}:", operands.len());

	for op in &operands {
		println!("- {:?}", op);
	}

	let mut tree = ByteTrie::Empty;

	for op in &ops {
		//println!("Inserting {} {}", table::bytes(&op.bytes), op.name);
		let opcode = tree.get_new(&op.bytes, Vec::new());

		let ops = map_ops(&op);
		let op_i = operands.iter().position(|r| r == &ops).unwrap();

		let inst = Inst(op.name.clone(), op_i);

		let rm = op.operands.iter()
			.find(|o| match o { &&(Operand::RmOpcode(..), _, _) => true, _ => false } )
			.map(|o| match o { &(Operand::RmOpcode(o), _, _) => o, _ => panic!() } );

		match opcode {
			&mut ByteTrie::Empty => {
				*opcode = ByteTrie::Decoded(if let Some(opcode) = rm {
					assert!(op.prefix_bytes.is_empty());
					let mut hm = HashMap::new();
					hm.insert(opcode as u8, inst);
					Multiplexer::Opcode(hm)
				} else {
					let mut hm = HashMap::new();
					hm.insert(op.prefix_bytes.to_vec(), inst);
					Multiplexer::Prefix(hm)
				});
			}
			&mut ByteTrie::Decoded(ref mut multi) => {
				match multi {
					&mut Multiplexer::Opcode(ref mut hm) => {
						if let Some(opcode) = rm {
							if hm.insert(opcode as u8, inst).is_some() {
								panic!("Existing opcode found in opcode multiplexer");
							}
						} else {
							panic!("Existing opcode multiplexer found");
						}
					}
					&mut Multiplexer::Prefix(ref mut hm) => {
						if rm.is_some() {
							panic!("Existing non-opcode multiplexer found");
						} else {
							if hm.insert(op.prefix_bytes.to_vec(), inst).is_some() {
								panic!("Existing prefix found in prefix multiplexer");
							}
						}
					}
				}
			},
			_ => panic!(),
		}
	}

	//println!("Tree: {:#?}", tree);

	let mut output = File::create(&Path::new("x86_opcodes.rs")).unwrap();

	output.write_all("pub fn decode(mut bytes: &[u8]) -> Option<usize> { Some({".as_bytes()).unwrap();
	tree.write(&mut output);
	output.write_all("})}".as_bytes()).unwrap();

	Command::new("rustfmt").arg("x86_opcodes.rs").output().unwrap();

	println!("Done!");
}
