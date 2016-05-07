use table;
use std::cmp;
use std::ptr;
use decoder;
use x86_opcodes;

static debug: bool = cfg!(debug_assertions);

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

pub fn inst(c: &mut Cursor) -> Option<(usize, Option<i64>, bool)> {
	let start_offset = c.offset;

	let gs_override = c.matches(table::P_SEG_GS);

	let mut prefixes = 0;

	if c.matches(table::P_OP_SIZE) {
		prefixes |= 8;
	}
	if c.matches(table::P_LOCK) {
		prefixes |= 1;
	}
	if c.matches(table::P_REP) {
		prefixes |= 2;
	}
	if c.matches(table::P_REPNE) {
		prefixes |= 4;
	}
	let operand_size_override = prefixes & 8 != 0;

	let segment_override = gs_override;
	
	let rex = c.peek() as u32;
	let rex = match rex {
		0x40...0x4F => {
			c.next();
			rex
		}
		_ => 0
	};

	let mut format = x86_opcodes::decode(c, prefixes) as u32;

	// Ensure prefixes are legal
	if !prefixes | (format & 0xF) != !0 {
		return None;
	}

	format >>= 4;

	// ModRM byte
	if format & 0x2000 != 0 {
		let modrm = c.next() as u32;
		let mode = modrm >> 6;
		let reg = ((modrm >> 3) & 7) | ((rex & 4) << 1);
		let rm_norex = modrm & 7;

		//println!("mode:{} reg:{} rm: {}", mode ,reg ,rm);

		let mut name = if mode != 3 && rm_norex == 4 {
			// Parse SIB byte

			let sib = c.next() as u32;
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

		match mode {
			0 | 3 => /*name.offset*/0,
			1 => c.next() as i8 as i32,
			2 => c.next_u32() as i32,
			_ => panic!(),
		};
	}

	let rex_w = rex & 8 != 0;

	// Opsize - 3 bits
	let op_size = match format & 7 {
		0 => 1,
		1 => if rex_w { 8 } else { if operand_size_override { 2 } else { 4 } },
		2 => 2,
		3 => 8,
		4 => 16,
		_ => panic!(),
	};
	format >>= 3;

	println!("(Imm {}, opsize = {}, operand_size_override = {})", format & 3, op_size, operand_size_override);

	// Imm type - 2 bits
	match format & 3 {
		0 => (),
		1 => c.offset += 1,
		2 => c.offset += cmp::min(op_size, 4),
		3 => c.offset += cmp::min(op_size, 8),
		_ => panic!(),
	};
	format >>= 2;
	// TODO: Check that c.offset is inbound

	let case = format & 0x1F;
	format >>= 5;

	println!("(Case {})", case);

	let result = match case {
		// Illegal
		0 => None,
		// Push
		9 => {
			Some((None, false))
		}
		// Pop
		10 => {
			Some((None, false))
		}
		// Call32
		14 => { 
			if segment_override {
				None
			} else {
				let offset = c.next_u32() as i32 as i64;
				Some((None, false))
			}
		}
		// Jmp32 | Jcc32
		15 | 21 => { 
			if segment_override {
				None
			} else {
				let offset = c.next_u32() as i32 as i64;
				Some((Some(offset), case == 15))
			}
		}
		// Jmp8 | Jcc8
		16 | 20 => {
			if segment_override {
				None
			} else {
				let offset = c.next() as i8 as i64;
				Some((Some(offset), case == 16))
			}
		}
		// Ud2
		17 => { 
			Some((None, true))
		}
		// Ret
		19 => { 
			if segment_override {
				None
			} else {
				Some((None, true))
			}
		}
		_ => Some((None, false)),
	};

	let len = c.offset - start_offset;

	if len >= 16 {
		return None;
	}

	result.map(|(j, t)| (len, j, t))
}

pub fn decode(data: &[u8], func_start: usize, func_size: usize, disp_off: u64) {
	let mut targets = Vec::new();
	let cp = decoder::capstone_open();
	targets.push(func_start as u64);

	println!("Disassembly:");

	let mut i = 0;

	while i < targets.len() {
		let mut c = Cursor {
			data: data,
			offset: targets[i] as usize,
		};

		println!("Label:");

		loop {
			let start = c.offset;
			let address = start as u64 + disp_off;
			print!("{:#08x}: ", address);

			if c.offset - func_start >= func_size {
				println!("ERROR: Instruction went outside function");
				panic!("Instruction went outside function");
			}

			let cs_data = &c.remaining()[0..cmp::min(16, c.remaining().len())];
			let (cs_desc, cs_len) = decoder::capstone_simple(cs_data, address).unwrap_or(("invalid".to_string(), 1));
			let cs_bytes = table::bytes(&c.remaining()[0..cs_len]);

			let mut str = String::new();

			let byte_print_len = cmp::min(8, cs_len);

			for b in c.data[start..(start + byte_print_len)].iter() {
				str.push_str(&format!("{:02x}", b));
			}

			for _ in 0..(8 - byte_print_len) {
				str.push_str("  ");
			}
			str.push_str(" ");

			print!("{}", str);

			println!("{: <40}", cs_desc);

			match inst(&mut c) {
				Some((len, jmp, term)) => {
					if len != cs_len {
						println!("Capstone length was {}, while the decoded instruction was {}", cs_len, len);
						panic!("Instruction length mismatch");
					}
					if let Some(target) = jmp {
						let off = (address + len as u64).wrapping_add(target as u64);
						println!("Jump target {:#x}", off);
						if off >= (disp_off + func_start as u64) && off < disp_off + (func_start + func_size) as u64 {
							let real_off = off - disp_off;
							if let Err(i) = targets.binary_search(&real_off) {
								println!("Inserting target {:#x}", real_off);
								targets.insert(i, real_off);
							}
						} else {
							panic!("Jump outside of symbol {:#x} ({:#x} - {:#x})", off, disp_off + func_start as u64, disp_off + func_start as u64 + func_size as u64);
						}
					}
					if term {
						break
					}
				}
				None => {
					println!("Illegal instruction");
					panic!("Illegal instruction");
				}
			}
		}

		i += 1;
	}

	decoder::capstone_close(cp);
}
