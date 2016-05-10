use table;
use std::cmp;
use std::ptr;
use decoder;
use x86_opcodes;
use std::collections::HashSet;

pub static DEBUG: bool = cfg!(debug_assertions);

#[derive(Clone, Debug)]
pub enum DecoderError {
	OutofBounds,
	SegmentOverrideOnBranch,
	InvalidPrefixes,
	UnknownInstruction,
	InstructionTooLong,
	SegmentedStackAccess,
	NonSegmentedMemAccess,
	ComplexAdressing,
	AbsoluteAdressing,
	StackIsNotRestored,
	MismatchedPop,
	TooManyPops,
	JumpOutsideOfFunction,
}

impl From<CursorError> for DecoderError {
    fn from(e: CursorError) -> DecoderError {
        DecoderError::OutofBounds
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
#[repr(packed)]
pub struct Reg(u8);

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum Operation {
	ClobReg(Reg),
	ClobStack(i32, u8),
	MoveRegs(Reg, Reg),
	MoveToStack(i32, Reg),
	MoveFromStack(Reg, i32),
	AndRegFromReg(Reg, Reg),
	AndRegFromStack(Reg, i32),
	AndStackFromReg(i32, Reg),
}

impl Operation {
	fn clobs_reg(&self) -> Option<Reg> {
		match *self {
			Operation::ClobReg(r) |
			Operation::MoveRegs(r, _) |
			Operation::AndRegFromReg(r, _) |
			Operation::AndRegFromStack(r, _) |
			Operation::MoveFromStack(r, _) => Some(r),
			Operation::ClobStack(..) |
			Operation::MoveToStack(..) |
			Operation::AndStackFromReg(..) => None,
		}
	}
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum Rm {
	None,
	Reg(Reg),
	Stack(i32),
	Base(Reg, i32),
}

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct Inst {
	jmp: Option<i64>,
	rm: Rm,
	term: bool,
}

pub struct CursorError;

#[derive(Copy, Clone)]
pub struct Cursor<'s> {
	pub data: &'s [u8],
	pub offset: usize,
}

impl<'s> Cursor<'s> {
	pub fn remaining(&self) -> &'s [u8] {
		if self.offset > self.data.len() {
			&[]
		} else {
			&self.data[self.offset..]
		}
	}

	pub fn peek(&self) -> Result<u8, CursorError> {
		match self.data.get(self.offset) {
			Some(&b) => Ok(b),
			None => return Err(CursorError)
		}
	}

	pub fn matches(&mut self, v: u8) -> Result<bool, CursorError> {
		Ok(if self.peek()? == v {
			self.next()?;
			true
		} else {
			false
		})
	}

	pub fn skip(&mut self, bytes: usize) -> Result<(), CursorError> {
		self.offset += bytes;
		if self.offset > self.data.len() {
			Err(CursorError)
		} else {
			Ok(())
		}
	}

	pub fn next(&mut self) -> Result<u8, CursorError> {
		let byte = self.peek()?;
		self.offset += 1;
		Ok(byte)
	}

	pub fn next_u32(&mut self) -> Result<u32, CursorError> {
		let mut v = self.next()? as u32;
		v |= (self.next()? as u32) << 8;
		v |= (self.next()? as u32) << 16;
		v |= (self.next()? as u32) << 24;
		Ok(v)
	}
}

pub struct FunctionState {
	clobs: HashSet<Reg>,
	callee_saved: Vec<Reg>,
	stack_offset: Option<(usize, usize)>,
	ops: Vec<Operation>,
}

impl FunctionState {
	fn op(&mut self, op: Operation) {
		if let Some(r) = op.clobs_reg() {
			self.clobs.insert(r);
		}
		self.ops.push(op);
	}
}

pub fn inst(c: &mut Cursor, state: &mut FunctionState) -> Result<(Inst, usize, usize), DecoderError> {
	fn def() -> Inst {
		Inst {
			jmp: None,
			rm: Rm::None,
			term: false,
		}
	}

	let ops_index = state.ops.len();

	let start_offset = c.offset;

	let gs_override = c.matches(table::P_SEG_GS)?;

	let mut prefixes = 0;

	if c.matches(table::P_OP_SIZE)? {
		prefixes |= 8;
	}
	if c.matches(table::P_LOCK)? {
		prefixes |= 1;
	}
	if c.matches(table::P_REP)? {
		prefixes |= 2;
	}
	if c.matches(table::P_REPNE)? {
		prefixes |= 4;
	}
	let operand_size_override = prefixes & 8 != 0;

	let segment_override = gs_override;
	
	let rex = c.peek()? as u32;
	let rex = match rex {
		0x40...0x4F => {
			c.next();
			rex
		}
		_ => 0
	};

	let mut format = x86_opcodes::decode(c, prefixes)? as u32;

	// Ensure prefixes are legal
	if !prefixes | (format & 0xF) != !0 {
		#[cfg(debug_assertions)]
		println!("(Invalid prefixes {} on {})", prefixes, format);
		return Err(DecoderError::InvalidPrefixes);
	}

	format >>= 4;

	let rex_w = rex & 8 != 0;

	// Opsize - 3 bits
	let op_size: u32 = match format & 7 {
		0 => 1,
		1 => if rex_w { 8 } else { if operand_size_override { 2 } else { 4 } },
		2 => 2,
		3 => 8,
		4 => 16,
		_ => panic!(),
	};
	format >>= 3;

	#[cfg(debug_assertions)]
	println!("(Imm {}, opsize = {}, operand_size_override = {})", format & 3, op_size, operand_size_override);

	let case = format & 0x1F;
	format >>= 5;

	#[cfg(debug_assertions)]
	println!("(Case {})", case);

	let modrm_ignore = |c: &mut Cursor| -> Result<Reg, DecoderError> {
		let modrm = c.next()? as u32;
		let mode = modrm >> 6;
		let reg = ((modrm >> 3) & 7) | ((rex & 4) << 1);
		let rm_norex = modrm & 7;

		let off = match mode {
			1 => c.skip(1)?,
			2 => c.skip(4)?,
			_ => (),
		};

		if mode != 3 {
			if rm_norex == 4 {
				// Parse SIB byte

				let sib = c.next()? as u32;
				let base_norex = sib & 7;
				if mode == 0 && base_norex == 5 {
					c.skip(4)?;
				}
			} else {
				if mode == 0 && rm_norex == 5 {
					c.skip(4)?
				}
			}
		}

		Ok(Reg(reg as u8))
	};

	let modrm = |c: &mut Cursor| -> Result<(Rm, Reg), DecoderError> {
		let modrm = c.next()? as u32;
		let mode = modrm >> 6;
		let reg = ((modrm >> 3) & 7) | ((rex & 4) << 1);
		let rm_norex = modrm & 7;

		//println!("mode:{} reg:{} rm: {}", mode ,reg ,rm);

		let off = match mode {
			0 | 3 => 0,
			1 => c.next()? as i8 as i32,
			2 => c.next_u32()? as i32,
			_ => panic!(),
		};

		let rm_rex = rm_norex | (rex & 1) << 3;

		let rm = if mode == 3 {
			Rm::Reg(Reg(rm_rex as u8))
		} else {
			if rm_norex == 4 {
				// Parse SIB byte

				let sib = c.next()? as u32;
				let base_norex = sib & 7;
				let index = ((sib >> 3) & 7) | (rex & 2) << 2;
				let scale = sib >> 6;

				if mode == 0 && base_norex == 5 {
					return Err(DecoderError::AbsoluteAdressing)
				}

				if index != 4 {
					return Err(DecoderError::ComplexAdressing);
				}

				let base_rex = base_norex | (rex & 1) << 3;

				if base_rex == 4 {
					if gs_override {
						return Err(DecoderError::SegmentedStackAccess)
					}
					Rm::Stack(off)
				} else {
					if !gs_override {
						return Err(DecoderError::NonSegmentedMemAccess)
					}
					Rm::Base(Reg(base_rex as u8), off)
				}
			} else {
				if mode == 0 && rm_norex == 5 {
					let _off = c.next_u32()? as i32;
					// TODO: Check that the offset with access size op_size is in the data segment
					Rm::None
				} else {
					if !gs_override {
						return Err(DecoderError::NonSegmentedMemAccess)
					}
					Rm::Base(Reg(rm_rex as u8), off)
				}
			}
		};

		Ok((rm, Reg(reg as u8)))
	};

	let reg_rex = |format| Reg(((format & 7) | (rex & 1) << 3) as u8);

	let result = match case {
		// Illegal
		0 => Err(DecoderError::UnknownInstruction),
		// WriteRm
		1 => {
			let (rm, _) = modrm(c)?;

			match rm {
				Rm::Stack(s) => state.op(Operation::ClobStack(s, op_size as u8)),
				Rm::Reg(r) => state.op(Operation::ClobReg(r)),
				_ => ()
			};

			Ok(Inst {
				rm: rm,
				..def()
			})
		}
		// ReadRmToReg
		2 => {
			let (rm, reg) = modrm(c)?;

			state.op(Operation::ClobReg(reg));

			Ok(Inst {
				rm: rm,
				..def()
			})
		}
		// ReadRm
		3 => {
			let (rm, reg) = modrm(c)?;
			Ok(Inst {
				rm: rm,
				..def()
			})
		}
		// Store
		4 => {
			let (rm, reg) = modrm(c)?;

			match rm {
				Rm::Stack(s) => state.op(if op_size == 8 {
					Operation::MoveToStack(s, reg)
				} else {
					Operation::ClobStack(s, op_size as u8)
				}),
				Rm::Reg(r) => state.op(if op_size == 8 {
					Operation::MoveRegs(reg, r)
				} else {
					Operation::ClobReg(r)
				}),
				_ => (),
			};

			Ok(Inst {
				rm: rm,
				..def()
			})
		}
		// Load
		5 => {
			let (rm, reg) = modrm(c)?;

			state.op(match rm {
				Rm::Stack(s) if op_size == 8 => Operation::MoveFromStack(reg, s),
				Rm::Reg(r) if op_size == 8 => Operation::MoveRegs(r, reg),
				_ => Operation::ClobReg(reg),
			});

			Ok(Inst {
				rm: rm,
				..def()
			})
		}
		// AndRmFromReg
		6 => {
			let (rm, reg) = modrm(c)?;

			match rm {
				Rm::Stack(s) => state.op(if op_size == 8 {
					Operation::AndStackFromReg(s, reg)
				} else {
					Operation::ClobStack(s, op_size as u8)
				}),
				Rm::Reg(r) => state.op(if op_size == 8 {
					Operation::AndRegFromReg(r, reg)
				} else {
					Operation::ClobReg(r)
				}),
				_ => (),
			};

			Ok(Inst {
				rm: rm,
				..def()
			})
		}
		// AndRmToReg
		7 => {
			let (rm, reg) = modrm(c)?;

			state.op(match rm {
				Rm::Stack(s) if op_size == 8 => Operation::AndRegFromStack(reg, s),
				Rm::Reg(r) if op_size == 8 => Operation::AndRegFromReg(reg, r),
				_ => Operation::ClobReg(reg),
			});

			Ok(Inst {
				rm: rm,
				..def()
			})
		}
		// Lea
		8 => {
			let reg = modrm_ignore(c)?;

			state.op(Operation::ClobReg(reg));

			Ok(Inst {
				..def()
			})
		}
		// Push
		9 => {
			let reg = reg_rex(format);

			match state.stack_offset {
				None if state.clobs.contains(&reg) => state.stack_offset = Some((1, state.callee_saved.len())),
				None => state.callee_saved.push(reg),
				Some((offset, saved)) => state.stack_offset = Some((offset + 1, saved)),
			}

			Ok(Inst {
				..def()
			})
		}
		// Pop
		10 => {
			let reg = reg_rex(format);

			match state.stack_offset {
				None => if let Some(r) = state.callee_saved.pop() {
					if reg != r {
						return Err(DecoderError::MismatchedPop);
					}
					state.clobs.remove(&r);
				} else {
					return Err(DecoderError::TooManyPops)
				},
				Some((0, 0)) => return Err(DecoderError::TooManyPops),
				Some((0, saved)) => {
					let i = saved - 1;
					if reg != state.callee_saved[i] {
						return Err(DecoderError::MismatchedPop);
					}
					state.stack_offset = Some((0, i));
				}
				Some((offset, saved)) => state.stack_offset = Some((offset - 1, saved)),
			}

			Ok(Inst {
				..def()
			})
		}
		// ClobRegRex
		11 => {
			let reg = reg_rex(format);

			state.op(Operation::ClobReg(reg));

			Ok(Inst {
				..def()
			})
		}
		// CheckAddr
		12 => panic!(),
		// CallRm
		13 => {
			// TODO: CFI
			modrm_ignore(c)?;
			Ok(Inst {
				..def()
			})
		}
		// Call32
		14 => { 
			if segment_override {
				Err(DecoderError::SegmentOverrideOnBranch)
			} else {
				let offset = c.next_u32()? as i32 as i64;
				Ok(Inst {
					..def()
				})
			}
		}
		// Jmp32 | Jcc32
		15 | 21 => { 
			if segment_override {
				Err(DecoderError::SegmentOverrideOnBranch)
			} else {
				let offset = c.next_u32()? as i32 as i64;
				Ok(Inst {
					jmp: Some(offset),
					term: case == 15,
					..def()
				})
			}
		}
		// Jmp8 | Jcc8
		16 | 20 => {
			if segment_override {
				Err(DecoderError::SegmentOverrideOnBranch)
			} else {
				let offset = c.next()? as i8 as i64;
				Ok(Inst {
					jmp: Some(offset),
					term: case == 16,
					..def()
				})
			}
		}
		// Ud2
		17 => { 
			Ok(Inst {
				term: true,
				..def()
			})
		}
		// None
		18 => {
			Ok(Inst {
				..def()
			})
		}
		// Ret
		19 => { 
			if segment_override {
				Err(DecoderError::SegmentOverrideOnBranch)
			} else {
				match state.stack_offset {
					None if state.callee_saved.is_empty() => (),
					Some((0, 0)) => (),
					_ => return Err(DecoderError::StackIsNotRestored),
				};

				Ok(Inst {
					term: true,
					..def()
				})
			}
		}
		// 20 => Jmp8 | Jcc8
		// 21 => Jmp32 | Jcc32
		// XchgRm
		22 => {
			let (rm, reg) = modrm(c)?;

			state.op(Operation::ClobReg(reg));

			match rm {
				Rm::Stack(s) => state.op(Operation::ClobStack(s, op_size as u8)),
				Rm::Reg(r) =>  state.op(Operation::ClobReg(r)),
				_ => ()
			};

			Ok(Inst {
				rm: rm,
				..def()
			})

		}
		_ => panic!(),
	};

	format >>= 3;

	// Imm type - 2 bits
	match format & 3 {
		0 => (),
		1 => c.skip(1)?,
		2 => c.skip(cmp::min(op_size, 4) as usize)?,
		3 => c.skip(cmp::min(op_size, 8) as usize)?,
		_ => panic!(),
	};
	format >>= 2;

	let len = c.offset - start_offset;

	if len >= 16 {
		return Err(DecoderError::InstructionTooLong);
	}

	result.map(|i| (i, len, ops_index))
}

pub static mut INSTRUCTIONS: usize = 0;

pub fn decode(data: &[u8], disp_off: u64) -> Result<(), DecoderError> {

	let mut state = FunctionState {
		clobs: HashSet::new(),
		callee_saved: Vec::new(),
		stack_offset: None,
		ops: Vec::new(),
	};

	let mut targets = Vec::new();
	targets.push(0 as u64);

	#[cfg(debug_assertions)]
	println!("Disassembly:");

	let mut i = 0;

	while i < targets.len() {
		let mut c = Cursor {
			data: data,
			offset: targets[i] as usize,
		};

		#[cfg(debug_assertions)]
		println!("Label:");

		loop {
			unsafe { INSTRUCTIONS += 1 };

			let start = c.offset;
			let address = start as u64 + disp_off;

			#[cfg(debug_assertions)]
			let cs_data = &c.remaining()[0..cmp::min(16, c.remaining().len())];

			#[cfg(debug_assertions)]
			let (cs_desc, cs_len) = decoder::capstone_simple(cs_data, address).unwrap_or(("invalid".to_string(), 0));
			
			#[cfg(debug_assertions)]
			{
				print!("{:#08x}: ", address);

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
			}

			let (inst, len, ops_index) = inst(&mut c, &mut state)?;

			#[cfg(debug_assertions)]
			{
				if len != cs_len {
					println!("Capstone length was {}, while the decoded instruction was {}", cs_len, len);
					panic!("Instruction length mismatch");
				}
			}

			if let Some(target) = inst.jmp {
				// Freeze the callee-saved registers
				if state.stack_offset.is_none() {
					state.stack_offset = Some((0, state.callee_saved.len()))
				}

				let off = (address + len as u64).wrapping_add(target as u64);
				#[cfg(debug_assertions)]
				println!("Jump target {:#x}", off);
				if off >= disp_off && off < disp_off + data.len() as u64 {
					let real_off = off - disp_off;
					if let Err(i) = targets.binary_search(&real_off) {
						#[cfg(debug_assertions)]
						println!("Inserting target {:#x}", real_off);
						targets.insert(i, real_off);
					}
				} else {
					#[cfg(debug_assertions)]
					println!("Jump outside of symbol {:#x} at {:#x}", off, address);
					return Err(DecoderError::JumpOutsideOfFunction);
				}
			}

			if inst.term {
				break
			}
		}

		i += 1;
	}

	println!("Done with function");

	for reg in state.clobs {
		if state.callee_saved.contains(&reg) {
			continue;
		}
		println!("Function clobbered {:?}", reg);
	}

	Ok(())
}
