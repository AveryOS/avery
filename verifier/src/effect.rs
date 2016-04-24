#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Size {
	Lit1,
	S8,
	S16,
	S32,
	S64,
	S128,
	SRexSize, // S32 without REX_W, S64 with
	SMMXSize,
	SImmSize,
	SOpSize,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Regs {
	GP,
	MMX,
	SSE,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Operand {
	Imm(Size),
	FixImm(i64, Size),
	Disp(Size),
	FixReg(usize, Regs),
	FixRegRex(usize, Regs),
	Clob(usize),
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

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RT {
	GP(usize),
	SSE(usize),
	CR(usize),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecodedOperand {
	Direct(RT),
	Indirect(IndirectAccess),
	Imm(i64, Size),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Inst {
	pub prefix_bytes: Vec<u8>,
	pub bytes: Vec<u8>,
	pub opcode: Option<usize>,
	pub operands: Vec<(Operand, Size)>,
	pub decoded_operands: Vec<(DecodedOperand, Size)>,
	pub op_size_postfix: bool,
	pub name: String,
	pub read_only: bool,
	pub no_mem: bool,
	pub unknown_mem: bool,
	pub prefix_whitelist: Vec<u8>,
	pub desc: String,
	pub operand_size: Size,
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

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
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
	pub fn trailing_bytes(self) -> usize {
		match self {
			Mem::Rip => 4,
			Mem::Mem(_, Disp::None) => 0,
			Mem::Mem(_, Disp::Imm8) => 1,
			Mem::Mem(_, Disp::Imm32) => 4,
		}
	}
}

// Store in 3 bits
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum StackMem {
	MemRSP(Disp),
	MemRBP(Disp),
}

// 10 bits for Mem + usize
// 6 bits for rest
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Effect {
	None,
	ClobReg(usize),
	WriteMem(Mem),
	CheckMem(Mem),
	Move(usize, usize),
	Store(Mem, usize), // Can be limited to RBP/RSP accesses (StackMem)
	Load(usize, Mem), // Can be limited to RBP/RSP accesses (StackMem)
	Lea(Mem),
	Push(usize),
	Pop(usize),
	CheckAddr,
	Imm64,
	Imm32,
	Imm16,
	Imm8,
	Call(Mem),
	Call32,
	Jmp32,
	Jmp8,
}

impl Effect {
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
			Effect::WriteMem(mem) |
			Effect::CheckMem(mem) |
			Effect::Store(mem, _) |
			Effect::Load(_, mem) |
			Effect::Lea(mem)  |
			Effect::Call(mem) => mem.trailing_bytes(),
			_ => 0,
		}
	}
}
