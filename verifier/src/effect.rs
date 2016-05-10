#[derive(Eq, PartialEq, Copy, Clone, Debug, PartialOrd, Ord)]
pub enum Size {
	Lit1,
	S8,
	S16,
	S32,
	S64,
	S128,
	SRexSize, // S32 without REX_W, S64 with
	SImmSize,
	SOpSize,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, PartialOrd, Ord)]
pub enum Regs {
	GP,
	SSE,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, PartialOrd, Ord)]
pub enum Access {
	Read,
	Write,
}

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum Operand {
	Imm(Size),
	FixImm(i64, Size),
	Disp(Size),
	FixReg(usize, Regs),
	FixRegRex(usize, Regs),
	Addr,
	Rm(Regs),
	Reg(Regs),
	RmOpcode(usize),
	Mem(Option<usize>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndirectAccess {
	pub base: Option<usize>,
	pub index: Option<usize>,
	pub scale: usize,
	pub offset: i64,
	pub offset_wide: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum RT {
	GP(usize),
	SSE(usize),
	CR(usize),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct IndirectAccessFormat {
	pub base: Option<usize>,
	pub index: Option<usize>,
	pub scale: usize,
	pub disp: Disp,
}

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum OperandFormat {
	Direct(RT, Size),
	Indirect(IndirectAccessFormat, Size),
	IndirectAddr,
	FixImm(i64),
	Imm(Size),
	Disp(Size),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstFormat {
	pub bytes: Vec<u8>,
	pub prefixes: Vec<u8>,
	pub prefix_bytes: Vec<u8>,
	pub operands: Vec<(OperandFormat, Access)>,
	pub name: String,
	pub no_mem: bool,
	pub op_size: Size,
	pub rex: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Inst {
	pub prefix_bytes: Vec<u8>,
	pub bytes: Vec<u8>,
	pub opcode: Option<usize>,
	pub accesses: Vec<(usize, Access)>,
	pub operands: Vec<(Operand, Size, Access)>,
	pub decoded_operands: Vec<(DecodedOperand, Size)>,
	pub op_size_postfix: bool,
	pub name: String,
	pub no_mem: bool,
	pub prefix_whitelist: Vec<u8>,
	pub desc: String,
	pub operand_size: Size,
	pub len: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecodedOperand {
	Direct(RT),
	Indirect(IndirectAccess),
	Imm(i64, Size),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedInst {
	pub operands: Vec<(DecodedOperand, Size)>,
	pub desc: String,
	pub name: String,
	pub len: usize,
}

/*
	SIB can be ignored since it isn't useful

	Effects:
	enum Disp {
		None,
		D8(i8),
		D32(i32),
	}
	enum RM {
		RM_RIP
		RM_Reg(GP)
		RM_M(GP, Disp)
	}

	Other_RM(RM) // Ignore Reg case? Check memory
	GP_RM_R(RM) // Clob register or check memory
	GP_R_RM(RM) // Clob register and check memory
	READ_RM(RM) // Check memory
	CALL_RM(RM) // Check call
	MOV_RM_R(RM) // Move register or move stack / check memory
	MOV_R_RM(RM) // Move register and move stack / check memory
	LEA_GP(RM) // Ignore RIP and RM_REG?

	Imm32,
	Imm16,
	Imm8,

	// Just one terminating case for all these?
	Jmp8
	Jmp32
	Ret

*/

// 16 + 7 = 23 values - Can store 3 of them in 16-bits

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum Disp {
	None,
	Imm8,
	Imm32,
}

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

// Store in 2 bits
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum StackMem {
	MemRSP(Disp),
}

// 10 bits for Mem + usize
// 6 bits for rest
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Effect {
	None,
	ClobReg(usize),
	CheckMem(Mem), // gs: memory access
	Move(usize, usize),
	WriteStack(Mem), // Can be limited to RSP accesses (StackMem) (Needs RIP too, RIP can be an additional case?) // Needs opsize too (or does it?, maybe for SSE only?)
	ReadStack(Mem), // Can be limited to RSP accesses (StackMem) (Needs RIP too, RIP can be an additional case?) // Needs opsize too (or does it?, maybe for SSE only?)
	Store(Mem, usize), // Can be limited to RSP accesses (StackMem) 2 + 4 = 6 bits
	Load(usize, Mem), // Can be limited to RSP accesses (StackMem) 2 + 4 = 6 bits
	//Lea(usize, Mem), // Needed? 6 + 4 = 10 bits
	Push(usize),
	Pop(usize),
	CheckAddr,
	Call(Mem),
	Call32,
	Jmp32,
	Jmp8,
	Ud2,

	// Can be used in addition to the cases above

	Imm64,
	Imm32,
	Imm16,
	Imm8,
}

impl Effect {
	pub fn encode(list: &[Effect]) -> usize {
		let first = if let Some(first) = list.first() {
			first
		} else {
			return 0;
		};

		// data: 10 bits
		// case: 4 bits
		let (case, data) = match *first {
			Effect::None => panic!(),

			Effect::CheckMem(mem) => (1, mem.encode()),
			Effect::WriteStack(mem) => (2, mem.encode()),
			Effect::ReadStack(mem) => (3, mem.encode()),
			Effect::Store(mem, r) => (4, mem.encode() | r << 6), 
			Effect::Load(r, mem) => (5, mem.encode() | r << 6),
			Effect::Push(r) => (6, r),
			Effect::Pop(r) => (7, r),
			Effect::CheckAddr => (8, 0),
			Effect::Call(mem) => (9, mem.encode()),
			Effect::Call32 => (10, 0),
			Effect::Jmp32 => (11, 0),
			Effect::Jmp8 => (12, 0),
			Effect::Ud2 => (13, 0),
			Effect::Move(f, t) => (14, f << 4 | t),

			// No main case
			Effect::ClobReg(..) |
			Effect::Imm8 |
			Effect::Imm16 |
			Effect::Imm32 |
			Effect::Imm64 => (0, 0),
		};

		let rem_list = if case != 0 {
			&list[1..list.len()]
		} else {
			list
		};

		// 4 bits for the rest
		let (rest, rest_data) = match rem_list {
			[] => (0, 0),
			//[Effect::Imm64] => 1,
			[Effect::Imm32] => (2, 0),
			[Effect::Imm16] => (3, 0),
			[Effect::Imm8] => (4, 0),
			[Effect::ClobReg(r)] => (5, r),
			[Effect::ClobReg(r), Effect::Imm8] => (6, r),
			[Effect::ClobReg(r), Effect::Imm16] => (7, r),
			[Effect::ClobReg(r), Effect::Imm32] => (8, r),
			[Effect::ClobReg(r), Effect::Imm64] => (9, r),
			[Effect::ClobReg(0), Effect::ClobReg(2)] => (10, 0),
			_ => panic!("Don't know how to encode rest {:?}", rem_list),
		};

		let rest = rest | rest_data << 4;

		let encoded = case | data << 4 | rest << 14 | rest << 14;

		let decoded = Effect::decode(encoded);

		if list != &decoded[..] {
			panic!("Encoding of {:?} ({:x}) decoded into {:?}", list, encoded, decoded);
		}

		encoded
	}

	pub fn decode(mut list: usize) -> Vec<Effect> {
		let mut result = Vec::new();
		let case = list & 0xF; // 4 bits
		list = list >> 4;

		let mem = list & 0x3F; // 6 bits;

		result.push(match case {
			1 => Effect::CheckMem(Mem::decode(mem)),
			2 => Effect::WriteStack(Mem::decode(mem)),
			3 => Effect::ReadStack(Mem::decode(mem)),
			4 => Effect::Store(Mem::decode(mem), (list >> 6) & 0xF), 
			5 => Effect::Load((list >> 6) & 0xF, Mem::decode(mem)),
			6 => Effect::Push(list & 0xF),
			7 => Effect::Pop(list & 0xF),
			8 => Effect::CheckAddr,
			9 => Effect::Call(Mem::decode(mem)),
			10 => Effect::Call32,
			11 => Effect::Jmp32,
			12 => Effect::Jmp8,
			13 => Effect::Ud2,
			14 => Effect::Move((list >> 4) & 0xF, list & 0xF),

			0 => Effect::None,

			_ => panic!("Unknown main case {:?}", case),
		});

		list = list >> 10;

		let case = list & 0xF;
		list = list >> 4;

		let rest = match case {
			0 => (),
			//[Effect::Imm64] => 1,
			2 => result.push(Effect::Imm32),
			3 => result.push(Effect::Imm16),
			4 => result.push(Effect::Imm8),
			5 => result.push(Effect::ClobReg(list & 0xF)),
			6 => {
				result.push(Effect::ClobReg(list & 0xF));
				result.push(Effect::Imm8);
			}
			7 => {
				result.push(Effect::ClobReg(list & 0xF));
				result.push(Effect::Imm16);
			}
			8 => {
				result.push(Effect::ClobReg(list & 0xF));
				result.push(Effect::Imm32);
			}
			9 => {
				result.push(Effect::ClobReg(list & 0xF));
				result.push(Effect::Imm64);
			}
			10 => {
				result.push(Effect::ClobReg(0));
				result.push(Effect::ClobReg(2));
			}
			_ => panic!("Unknown rest case {:?}", case),
		};

		result.retain(|v| *v != Effect::None);

		result
	}

	pub fn sort_key(self) -> usize {
		match self {
			Effect::None |
			Effect::CheckMem(..) |
			Effect::WriteStack(..) |
			Effect::ReadStack(..) |
			Effect::Store(..) | 
			Effect::Move(..) | 
			Effect::Load(..) |
			Effect::Push(..) |
			Effect::Pop(..) |
			Effect::CheckAddr |
			Effect::Call(..) |
			Effect::Call32 |
			Effect::Jmp32 |
			Effect::Jmp8 |
			Effect::Ud2 => 0,

			Effect::ClobReg(..) => 1,

			Effect::Imm8 |
			Effect::Imm16 |
			Effect::Imm32 |
			Effect::Imm64 => 2,
		}
	}

	pub fn trailing_bytes(self) -> usize {
		match self {
			Effect::CheckAddr => 8,
			Effect::Imm8 => 1,
			Effect::Call32 => 4,
			Effect::Jmp32 => 4,
			Effect::Jmp8 => 1,
			Effect::Imm16 => 2,
			Effect::Imm32 => 4,
			Effect::Imm64 => 8,
			Effect::WriteStack(mem) |
			Effect::ReadStack(mem) |
			Effect::CheckMem(mem) |
			Effect::Store(mem, _) |
			Effect::Load(_, mem) |
			//Effect::Lea(_, mem)  |
			Effect::Call(mem) => mem.trailing_bytes(),
			_ => 0,
		}
	}
}
