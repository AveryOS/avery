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

use std::cell::Cell;
use std::process::Command;
use std::path::Path;
use std::fs::File;
use std::io::Write;
use std::collections::HashMap;

mod effect;
mod decoder;
mod table;
mod disasm;

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(packed)]
pub struct Reg(u8);

// 5 bits for cases, 3 for Reg = 8 bits
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum InstKind {
	Illegal,
	None,
	WriteRm,
	ReadRmToReg, // Reg <- Rm
	ReadRm,
	Store, // mov Rm, Reg
	Load, // mov Reg, Rm
	AndRmFromReg, // and Rm, Reg
	AndRmToReg, // and Reg, Rm
	Lea,
	Push(Reg),
	//PushRM,
	Pop(Reg),
	//PopRM,
	ClobRegRex(Reg),
	CheckAddr, // Is this useful? Maybe for 32-bit constants only? TODO: Disable this in LLVM
	CallRm,
	Call32,
	Jmp32,
	Jmp8,
	Jcc32,
	Jcc8,
	Ud2,
	Ret,
}

// 2 bits
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum Imm {
	None,
	Size8,
	Opsize32,
	Opsize64,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum Opsize {
	Size8,
	Size16,
	Size64,
	Size128,
	SizeDef,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(packed)]
pub struct InstFormat {
	kind: InstKind,
	opsize: Opsize,
	modrm: bool,
	clob_rax: bool,
	clob_rdx: bool,
	opsize_prefix: bool,
	rep_prefix: bool,
	repne_prefix: bool,
	lock_prefix: bool,
	imm: Imm,
}

fn bit(b: bool) -> u32 {
	if b { 1 } else { 0 }
}

impl InstKind {
	fn encode(self) -> u32 {
		match self {
			InstKind::Illegal => 0,
			InstKind::WriteRm => 1,
			InstKind::ReadRmToReg => 2,
			InstKind::ReadRm => 3,
			InstKind::Store => 4, 
			InstKind::Load => 5, 
			InstKind::AndRmFromReg => 6,
			InstKind::AndRmToReg => 7,
			InstKind::Lea => 8,
			InstKind::Push(r) => 9,
			InstKind::Pop(r) => 10,
			InstKind::ClobRegRex(r) => 11,
			InstKind::CheckAddr => 12, 
			InstKind::CallRm => 13,
			InstKind::Call32 => 14,
			InstKind::Jmp32 => 15,
			InstKind::Jmp8 => 16,
			InstKind::Ud2 => 17,
			InstKind::None => 18,
			InstKind::Ret => 19,
			InstKind::Jcc8 => 20,
			InstKind::Jcc32 => 21,
		}
	}
}

impl Imm {
	fn encode(self) -> u32 {
		match self {
			Imm::None => 0,
			Imm::Size8 => 1,
			Imm::Opsize32 => 2,
			Imm::Opsize64 => 3,
		}
	}
}

impl Opsize {
	fn encode(self) -> u32 {
		match self {
			Opsize::Size8 => 0,
			Opsize::SizeDef => 1,
			Opsize::Size16 => 2,
			Opsize::Size64 => 3,
			Opsize::Size128 => 4,
		}
	}
}

impl InstFormat {
	fn encode(self) -> u32 {
		let mut r = 0;
		let mut s = 0;

		if (self.lock_prefix && self.rep_prefix) ||
			(self.lock_prefix && self.repne_prefix) ||
			(self.rep_prefix && self.repne_prefix) {
			panic!("Mutually exclusive prefixes set!")
		}

		r |= bit(self.lock_prefix) << s;
		s += 1;

		r |= bit(self.rep_prefix) << s;
		s += 1;

		r |= bit(self.repne_prefix) << s;
		s += 1;

		r |= bit(self.opsize_prefix) << s;
		s += 1;

		r |= self.opsize.encode() << s;
		s += 3;
		
		r |= self.imm.encode() << s;
		s += 2;
		
		r |= self.kind.encode() << s;
		s += 8;

		r |= bit(self.modrm) << s;
		s += 1;
		
		r |= bit(self.clob_rax) << s;
		s += 1;

		r |= bit(self.clob_rdx) << s;
		s += 1;

		// Total bits = 19

		r
	}
}


use effect::{Size, Operand, Access, Regs};

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
struct Operands {
	pub operands: Vec<(Operand, Size, Access)>,
	pub mov: bool,
	pub lea: bool,
	pub call: bool,
	pub accesses: Vec<(usize, Access)>,
	//pub source: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Inst(String, usize);

impl Inst {
	fn note(&self, writer: &mut Write) {
		write!(writer, " /* {} */ \n", self.0).unwrap();
	}
	fn val(&self, writer: &mut Write) {
		write!(writer, "{:#02x}", self.1).unwrap();
	}
	fn write(&self, writer: &mut Write) {
		self.note(writer);
		self.val(writer);
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

	fn write(&self, writer: &mut Write, taken: Vec<u8>) {
		match *self {
			ByteTrie::Empty => panic!(),
			ByteTrie::Decoded(ref multi) => multi.write(writer, taken),
			ByteTrie::Map(ref map) => {
				writer.write_all("match c.next() { ".as_bytes()).unwrap();
				let mut entries = map.iter().collect::<Vec<_>>();
				entries.sort_by_key(|e| e.0);
				for (b, tree) in entries {
					let mut taken = taken.clone();
					taken.push(*b);
					write!(writer, "{:#02x} => {{ ", b).unwrap();
					tree.write(writer, taken);
					writer.write_all("}".as_bytes()).unwrap();
				}
				writer.write_all("_ => 0 }".as_bytes()).unwrap();
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
	fn write(&self, writer: &mut Write, taken: Vec<u8>) {
		match *self {
			Multiplexer::Opcode(ref map) => {
				let entries: Vec<_> = (0..8u8).map(|i| map.get(&i).map(|v| Ok(v)).unwrap_or_else(|| {
					let mut b = taken.clone();
					b.extend_from_slice(&[0x1A, 0x1A, 0x1A, 0x1A, 0x1A, 0x1A, 0x1A, 0x1A, 0x1A, 0x1A, 0x1A, 0x1A, 0x1A, 0x1A, 0x1A, 0x1A]);
					Err(decoder::capstone_simple(&b, 0))
				})).collect();

				let first = map.values().next().unwrap();

				let all_same = entries.iter().all(|e| {
					match *e {
						Ok(ref v) => v.1 == first.1,
						_ => false,
					}
				});

				if all_same {
					writer.write_all("\n// Opcode table\n".as_bytes()).unwrap();

					for (i, b) in entries.into_iter().enumerate() {
						match b {
							Ok(op) => write!(writer, "\n// {} => {} \n", i, op.0).unwrap(),
							Err(None) => write!(writer, "\n// {} => capstone: unknown \n", i).unwrap(),
							Err(Some((s, _))) => write!(writer, "\n// {} => capstone: {} \n", i, s).unwrap(),
						}
					}
					first.val(writer);

				} else {
					writer.write_all("match (c.peek() >> 3u8) & 7 { \n".as_bytes()).unwrap();

					for (i, b) in entries.into_iter().enumerate() {
						match b {
							Ok(op) => {
								write!(writer, "{:#02x} => {{", i).unwrap();
								op.write(writer);
								writer.write_all("}\n".as_bytes()).unwrap();
							}
							Err(None) => write!(writer, "// {} => capstone: unknown \n", i).unwrap(),
							Err(Some((s, _))) => write!(writer, "// {} => capstone: {} \n", i, s).unwrap(),
						}
					}
					writer.write_all("_ => 0 }".as_bytes()).unwrap();
				}
			}
			Multiplexer::Prefix(ref map) => {
				let mut entries = map.iter().collect::<Vec<_>>();
				entries.sort_by_key(|e| e.0);
				
				if map.len() == 1 && map.get(&[][..]).is_some() {
					(map.iter().next().unwrap().1).write(writer);
				} else {
					let first = map.values().next().unwrap();

					if map.values().all(|v| v.1 == first.1) {
						writer.write_all("\n// Multiple prefixes\n".as_bytes()).unwrap();

						for (b, i) in entries {
							i.note(writer);
						}
						first.val(writer);
					} else {
						for (i, &(prefixes, inst)) in entries.iter().enumerate() {
							if !prefixes.is_empty() {
								let bit = match &prefixes[..] {
									[table::P_REP] => 2,
									[table::P_REPNE] => 4,
									[table::P_OP_SIZE] => 8,
									_ => panic!(),
								};

								for (j, &(_, other_inst)) in entries.iter().enumerate() {
									if i == j {
										continue;
									}

									if bit & other_inst.1 != 0 {
										panic!("Prefixes are not mutually exclusive on {}", table::bytes(&taken));
									}
								}

								write!(writer, "if prefixes & {} != 0 {{ return ", bit).unwrap();
								inst.write(writer);
								writer.write_all("}".as_bytes()).unwrap();
							}
						}
						if let Some(i) = map.get(&[][..]) {
							i.write(writer);
						} else {
							writer.write_all("0".as_bytes()).unwrap();
						}
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
	let mut formats = Vec::new();

	let map_ops = |op: &effect::Inst| -> (Operands, InstFormat) {
		//println!("Mapping inst {:?}", op);
		let clob_rax = Cell::new(false);

		let ops: Vec<_> = op.operands.iter().filter(|o| {
			match **o {
				(Operand::FixReg(0, effect::Regs::GP), s, Access::Write) => {
					clob_rax.set(true);
					false
				}
				(Operand::FixReg(..), s, Access::Read) => false,
				(Operand::FixRegRex(..), s, Access::Read) => false,
				(Operand::FixImm(..), _, _) => false,
				_ => true,
			}
		}).map(|o| {
			match *o {
				(Operand::RmOpcode(..), s, a) => (Operand::Rm(Regs::GP), s, a),
				ref e => e.clone()
			}
		}).collect();

		let (modrm, mem_opsize) = ops.iter().map(|o| {
			match *o {
				(Operand::RmOpcode(..), s, _) |
				(Operand::Mem(..), s, _) |
				(Operand::Rm(..), s, _) => (true, s),
				_ => (false, Size::S8),
			}
		}).find(|&(m, _)| m).unwrap_or((false, op.operand_size));

		let opsize = match mem_opsize {
			Size::S8 => Opsize::Size8,
			Size::S16 => Opsize::Size16,
			Size::S32 if !op.prefix_whitelist.contains(&table::P_OP_SIZE) => Opsize::SizeDef,
			Size::SRexSize if !op.prefix_whitelist.contains(&table::P_OP_SIZE) => Opsize::SizeDef,
			Size::SOpSize => Opsize::SizeDef,
			Size::S64 => Opsize::Size64,
			Size::S128 => Opsize::Size128,
			_ => panic!("Unknown opsize {:?} for {:?}", op.operand_size, op),
		};

		let imm = ops.iter().map(|o| {
			match *o {
				(Operand::Imm(s), _, _) => Some(s),
				_ => None,
			}
		}).find(|v| v.is_some()).map(|v| v.unwrap());

		let imm = match imm {
			None => Imm::None,
			Some(s) => match s {
				Size::S8 => Imm::Size8,
				Size::SImmSize => Imm::Opsize32,
				Size::SOpSize => Imm::Opsize64,
				_ => panic!("Unmatched size {:?}", s),
			}
		};

		let full_mov = op.name == "mov" && (&op.bytes == &[0x89] || &op.bytes == &[0x8b]);

		let opers = Operands {
			mov: full_mov,
			lea: op.name == "lea",
			call: op.name == "call",
			accesses: op.accesses.clone(),
			operands: ops.clone(),
			//source: op.name.clone(),
		};

		let is_jump = match &ops[..] {
				[(Operand::Disp(s), _, _)] => true,
				_ => false,
		};

		let kind = if op.name == "ud2" {
			InstKind::Ud2 
		} else if op.name == "ret" {
			InstKind::Ret
		} else if is_jump {
			if op.name == "call" {
				match *ops.first().unwrap() {
					(Operand::Disp(_), _, _) => InstKind::Call32,
					(Operand::Rm(..), _, _) => InstKind::CallRm,
					_ => panic!(),
				}
			} else {
				match *ops.first().unwrap() {
					(Operand::Disp(s), _, _) => match s {
						Size::S8 => if op.name == "jmp" { InstKind::Jmp8 } else { InstKind::Jcc8 },
						Size::S32 => if op.name == "jmp" { InstKind::Jmp32 } else { InstKind::Jcc32 },
						_ => panic!(),
					},
					_ => panic!(),
				}
			}
		} else if op.name == "push" {
			match *ops.first().unwrap() {
				(Operand::FixRegRex(r, effect::Regs::GP), _, _) => InstKind::Push(Reg(r as u8)),
				_ => panic!(),
			}
		} else if op.name == "pop" {
			match *ops.first().unwrap() {
				(Operand::FixRegRex(r, effect::Regs::GP), _, _) => InstKind::Pop(Reg(r as u8)),
				_ => panic!(),
			}
		} else if op.name == "lea" {
			InstKind::Lea
		} else if full_mov {
			if &op.bytes == &[0x89] {
				InstKind::Store
			} else {
				InstKind::Load
			}
		} else {
			let ops: Vec<_> = ops.into_iter().filter(|o| {
				match *o {
					(Operand::Imm(_), _, _) => false,
					_ => true,
				}
			}).collect();

			let is_and = op.name == "and" && opsize == Opsize::SizeDef;

			match &ops[..] {
				[] => InstKind::None,
				[(Operand::Addr, _, _)] => InstKind::CheckAddr,
				[(Operand::FixRegRex(r, effect::Regs::GP), _, Access::Write)] => InstKind::ClobRegRex(Reg(r as u8)),
				[(Operand::Rm(..), _, Access::Read)] => InstKind::ReadRm,
				[(Operand::Rm(..), _, Access::Write)] => InstKind::WriteRm,
				[(Operand::Rm(..), _, Access::Read), (Operand::Reg(..), _, Access::Read)] => InstKind::ReadRm,
				[(Operand::Reg(..), _, Access::Read), (Operand::Rm(..), _, Access::Read)] => InstKind::ReadRm,
				[(Operand::Rm(..), _, Access::Write), (Operand::Reg(..), _, Access::Read)] => {
					if is_and {
						InstKind::AndRmFromReg
					} else {
						InstKind::WriteRm
					}
				}
				[(Operand::Reg(..), _, Access::Write), (Operand::Rm(..), _, Access::Read)] => {
					if is_and {
						InstKind::AndRmToReg
					} else {
						InstKind::ReadRmToReg
					}
				}
				_ => panic!("Unmatched operands {:?} for inst {:?}", ops, op),
			}
		};

		let f = InstFormat {
			kind: kind,
			opsize: opsize,
			modrm: modrm,
			rep_prefix: op.prefix_bytes.contains(&table::P_REP),
			repne_prefix: op.prefix_bytes.contains(&table::P_REPNE),
			lock_prefix: op.prefix_whitelist.contains(&table::P_LOCK),
			opsize_prefix: op.prefix_whitelist.contains(&table::P_OP_SIZE) || op.prefix_bytes.contains(&table::P_OP_SIZE),
			clob_rax: clob_rax.get() || op.accesses.contains(&(0, Access::Write)),
			clob_rdx: op.accesses.contains(&(2, Access::Write)),
			imm: imm,
		};

		(opers, f)
	};

	for op in &ops {
		let r = map_ops(&op);
		formats.push(r.1);
		operands.push(r.0);
	}

	formats.sort();
	formats.dedup();

	operands.sort();
	operands.dedup();

	println!("Formats {}:", formats.len());

	for op in &formats {
		println!("- {:?}", op);
	}

	println!("Operands {}:", operands.len());

	for op in &operands {
		println!("- {:?}", op);
	}

	let mut tree = ByteTrie::Empty;

	let insert_op = |tree: &mut ByteTrie, op, bytes: &[u8]| {
		let opcode = tree.get_new(bytes, Vec::new());

		let ops = map_ops(op).1;
		let op_i = ops.encode() as usize;

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
	};

	for op in &ops {
		//println!("Inserting {} {}", table::bytes(&op.bytes), op.name);
		if op.name == "nop" && &op.bytes == &[0x0f, 0x1f] {
			let mut bytes = op.bytes.clone();
			bytes.insert(0, table::P_SEG_CS);
			insert_op(&mut tree, op, &bytes);
			bytes.insert(0, table::P_OP_SIZE);
			insert_op(&mut tree, op, &bytes);
			bytes.insert(0, table::P_OP_SIZE);
			insert_op(&mut tree, op, &bytes);
			bytes.insert(0, table::P_OP_SIZE);
			insert_op(&mut tree, op, &bytes);
			bytes.insert(0, table::P_OP_SIZE);
			insert_op(&mut tree, op, &bytes);
		}

		insert_op(&mut tree, op, &op.bytes);
	}

	//println!("Tree: {:#?}", tree);

	let mut output = File::create(&Path::new("src/x86_opcodes.rs")).unwrap();

	output.write_all("use x86_decoder::Cursor;pub fn decode(c: &mut Cursor, prefixes: u32) -> u32 {".as_bytes()).unwrap();
	tree.write(&mut output, Vec::new());
	output.write_all("}".as_bytes()).unwrap();

	Command::new("rustfmt").arg("src/x86_opcodes.rs").output().unwrap();

	println!("Done!");
}
