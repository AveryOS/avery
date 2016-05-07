use table;
use std::cmp;
use std::ptr;
use effect::{Effect, DecodedOperand, Size, InstFormat, DecodedInst};
use disasm;
use decoder;
use x86_opcodes;

// Store in 6 bits - 2 for Rip/None/Imm8/Imm32 - 4 for register
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Mem {
	Rip,
	Mem(usize, Disp),
}

impl Mem {
	pub fn encode(self) -> usize {
		match self {
			Mem::Rip => 0,
			Mem::Mem(r, Disp::None) => 1 | r << 2,
			Mem::Mem(r, Disp::Imm8) => 2 | r << 2,
			Mem::Mem(r, Disp::Imm32) => 3 | r << 2,
		}
	}
	
	pub fn decode(val: usize) -> Self {
		let r = val >> 2;
		match val & 3 {
			0 => Mem::Rip,
			1 => Mem::Mem(r, Disp::None),
			2 => Mem::Mem(r, Disp::Imm8),
			3 => Mem::Mem(r, Disp::Imm32),
			_ => panic!(),
		}
	}
	
	pub fn trailing_bytes(self) -> usize {
		match self {
			Mem::Rip => 4,
			Mem::Mem(_, Disp::None) => 0,
			Mem::Mem(_, Disp::Imm8) => 1,
			Mem::Mem(_, Disp::Imm32) => 4,
		}
	}
}

pub enum Operation {
	ClobReg(u8),
	ClobStack(i32, u8),
	MoveRegs(u8, u8),
	MoveToStack(i32, u8),
	MoveFromStack(u8, i32),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct InstEffect {
	operation_index: usize,
}

#[derive(Copy, Clone)]
pub struct Cursor<'s> {
	pub data: &'s [u8],
	pub offset: usize,
}

impl<'s> Cursor<'s> {
	pub fn remaining(&self) -> &'s [u8] {
		&self.data[self.offset..]
	}

	pub fn peek(&self) -> u8 {
		self.data[self.offset]
	}

	pub fn matches(&mut self, v: u8) -> bool {
		if self.peek() == v {
			self.next();
			true
		} else {
			false
		}
	}

	pub fn next(&mut self) -> u8 {
		let byte = self.data[self.offset];
		self.offset += 1;
		byte
	}

	pub fn next_u32(&mut self) -> u32 {
		let mut v = self.next() as u32;
		v |= (self.next() as u32) << 8;
		v |= (self.next() as u32) << 16;
		v |= (self.next() as u32) << 24;
		v
	}

	pub fn next_u16(&mut self) -> u16 {
		let mut v = self.next() as u16;
		v |= (self.next() as u16) << 8;
		v
	}
	pub fn next_u64(&mut self) -> u64 {
		let mut v = self.next() as u64;
		v |= (self.next() as u64) << 8;
		v |= (self.next() as u64) << 16;
		v |= (self.next() as u64) << 24;
		v |= (self.next() as u64) << 32;
		v |= (self.next() as u64) << 40;
		v |= (self.next() as u64) << 48;
		v |= (self.next() as u64) << 56;
		v
	}
}

pub fn inst(c: &mut Cursor, disp_off: u64) -> Option<usize> {
	let s = c;

	let mut prefixes = 0;

	if c.matches(&table::P_LOCK) {
		prefixes |= 1;
	}
	if c.matches(&table::P_REP) {
		prefixes |= 2;
	}
	if c.matches(&table::P_REPNE) {
		prefixes |= 4;
	}
	if c.matches(&table::P_OP_SIZE) {
		prefixes |= 8;
	}

	let operand_size_override = prefixes | 8 != 0

	// 0xF2, 0xF3 and 0x66 can be part of instructions
	// 0xF2, 0xF3 must only be used when required

	// TODO: Disallow GS and FS prefixes with branching instructions
	let gs_override = c.matches(&table::P_SEG_GS);
	
	let rex = c.peek() as u32;
	let rex = match rex {
		0x40...0x4F => {
			c.next();
			rex
		}
		_ => 0
	};

	let mut format = x86_opcodes::decode(c.remaining(), prefixes) as u32;

	// Ensure prefixes are legal
	if !prefixes | (format & 0xF) != !0 {
		return None;
	}

	format >>= 4;

	// ModRM byte
	if format & 0x1000 != 0 {
		let modrm = c().next() as u32;
		let mode = modrm >> 6;
		let reg = ((modrm >> 3) & 7) | ((rex & 4) << 1);
		let rm_norex = modrm & 7;

		//println!("mode:{} reg:{} rm: {}", mode ,reg ,rm);

		let mut name = if mode != 3 && rm_norex == 4 {
			// Parse SIB byte

			let sib = c().next() as usize;
			let base_norex = sib & 7;
			let index = ((sib >> 3) & 7) | (rex & 2) << 2;
			let scale = sib >> 6;

			let reg_index = if index == 4 {
				None
			} else {
				Some(index)
			};
			let (reg_base, off) = if mode == 0 && base_norex == 5 {
				(None, c.next_u32())
			} else {
				let base_rex = base_norex | (rex & 1) << 3;
				(Some(base_rex), 0)
			};

			/*IndirectAccess {
				base: reg_base,
				index: reg_index,
				scale: 1 << scale,
				offset: off,
				offset_wide: false,
			}*/
		} else {
			if mode == 0 && rm_norex == 5 {
				// RIP relative

				let off = c.next_u32();

				/*IndirectAccess {
					base: Some(16), // RIP
					index: None,
					scale: 0,
					offset: off,
					offset_wide: false,
				}*/
			} else {
				let rm_rex = rm_norex | (rex & 1) << 3;

				/*IndirectAccess {
					base: Some(rm_rex),
					index: None,
					scale: 0,
					offset: 0,
					offset_wide: false,
				}*/
			}
		};

		let off = match mode {
			0 | 3 => name.offset,
			1 => c.next(),
			2 => c.next_u32(),
			_ => panic!(),
		};
	}

	let rex_w = rex & 8 != 0;

	// Opsize - 2 bits
	let op_size = if format & 1 != 0 {
		if rex_w { 8 } else { if operand_size_override { 2 } else { 4 } }
	} else {
		1
	};
	format >>= 2;

	// Imm type - 2 bits
	let imm = match format & 3 {
		0 => (),
		1 => c.offset += 1,
		2 => c.offset += cmp::min(op_size, 4),
		3 => c.offset += cmp::min(op_size, 8),
	};
	// TODO: Check that c.offset is inbound

	let case = format & 0x1F;
	format >>= 5;

	match format & 0x1F {

	}

	if r == 0 {
		let data = &s.remaining()[0..cmp::min(16, s.remaining().len())];
		let (desc, len) = decoder::capstone_simple(data, 0).unwrap_or(("invalid".to_string(), 1));
		let bytes = table::bytes(&s.remaining()[0..len]);

		println!("unknown |{}| capstone: {}", bytes, desc);
		panic!("unknown |{}| capstone: {}", bytes, desc);
	}

	// TOOD: Check that length is 15 bytes or lower

	(inst, case.1.clone())
}

pub fn decode(data: &[u8], func_start: usize, size: usize, disp_off: u64) {
	let mut targets = Vec::new();
	let mut cp = decoder::capstone_open();
	targets.push(func_start);

	let mut i = 0;

	while i < targets.len() {
		let mut c = Cursor {
			data: data,
			offset: targets[i],
		};

		println!("disasm:");

		loop {
			let start = c.offset;
			let address = start as u64 + disp_off;
			print!("{:#08x}: ", address);
			let cs_data = &c.remaining()[0..cmp::min(16, c.remaining().len())];
			let data = &s.remaining()[0..cmp::min(16, s.remaining().len())];
			let (cs_desc, cs_len) = decoder::capstone_simple(cs_data, 0).unwrap_or(("invalid".to_string(), 1));
			let cs_bytes = table::bytes(&s.remaining()[0..len]);

			let (i, effects) = inst(&mut c, address);
			let mut str = String::new();

			let byte_print_len = cmp::min(8, i.len);

			for b in c.data[start..(start + byte_print_len)].iter() {
				str.push_str(&format!("{:02x}", b));
			}

			for _ in 0..(8 - byte_print_len) {
				str.push_str("  ");
			}
			str.push_str(" ");

			print!("{}", str);

			println!("{: <40} {:?} ({:x}/{:x})", i.desc, effects, c.offset - func_start, size);
/*
			if capstone(&mut cp, cs_data, address, &i, &effects) {
					println!("unknown |{}| capstone: {}", bytes, desc);
					panic!("unknown |{}| capstone: {}", bytes, desc);
				panic!("Capstone output didn't match");
			}
*/
			if effects.iter().any(|o| match *o { Effect::Jmp32 | Effect::Jmp8 => true, _ => false }) {
				let op: (DecodedOperand, Size) = i.operands.first().unwrap().clone();
				let off = match op.0 {
					DecodedOperand::Imm(off, _) => {
						Some(off as u64)
					}
					_ => None,
				};
				if let Some(off) = off {
					let off = off as usize;
					if off >= start && off < start + size {
						if let Err(i) = targets.binary_search(&off) {
							targets.insert(i, off);
						}
					} else {
						//println!("Jump outside of symbol {:#x}", off);
					}
				}
			}

			if i.name == "jmp" || i.name == "ret" || i.name == "ud2" {
				break
			}

			if c.offset - func_start >= size {
				println!("ERROR: Instruction went outside function");
				panic!("Instruction went outside function");
			}
		}

		i += 1;
	}

	decoder::capstone_close(cp);
}
