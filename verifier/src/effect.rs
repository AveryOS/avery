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
pub enum Effect2 {
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

impl Effect2 {
	pub fn trailing_bytes(self) -> usize {
		match self {
			Effect2::CheckAddr => 8,
			Effect2::Imm8 => 1,
			Effect2::Call32 => 4,
			Effect2::Jmp32 => 4,
			Effect2::Jmp8 => 1,
			Effect2::Imm16 => 2,
			Effect2::Imm32 => 4,
			Effect2::Imm64 => 8,
			Effect2::WriteMem(mem) |
			Effect2::CheckMem(mem) |
			Effect2::Store(mem, _) |
			Effect2::Load(_, mem) |
			Effect2::Lea(mem)  |
			Effect2::Call(mem) => mem.trailing_bytes(),
			_ => 0,
		}
	}
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Effect {
	ClobRAX,
	ClobRDX,
	ClobReg(usize),
	ClobRM_R,
	ClobR_RM,
	MovRM_R,
	MovR_RM,
	ReadRM,
	ImmMatchOp,
	ImmOp,
	Imm8,
	CallRM,
	Call32,
	Jmp32,
	Jmp8,
	StackOp,
	Lea,
	ImmAddr,
}

impl Effect {
	pub fn trailing_bytes(self, op_size: bool) -> usize {
		let imm_size = if op_size { 2 } else { 4 };
		match self {
			Effect::ImmAddr => 8,
			Effect::Imm8 => 1,
			Effect::Call32 => 4,
			Effect::Jmp32 => 4,
			Effect::Jmp8 => 1,
			Effect::ImmOp => imm_size,
			Effect::ImmMatchOp => 4,
			Effect::ClobRM_R |
				Effect::ClobR_RM |
				Effect::MovRM_R |
				Effect::MovR_RM |
				Effect::ReadRM |
				Effect::CallRM |
				Effect::Lea => 5,
			_ => 0,
		}
	}

	pub fn need_rex_w(self) -> bool {
		match self {
			Effect::ImmAddr => true,
			_ => false,
		}
	}
}