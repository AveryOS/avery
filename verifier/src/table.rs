use effect::Inst;
use effect::Size;
use effect::Size::*;
use effect::Operand;
use effect::Regs;
use effect::Access;

pub static mut DEBUG: bool = false;

macro_rules! debug {
    ($($arg:tt)*) => (
        if unsafe { DEBUG } {
            print!($($arg)*);
        }
    );
}

fn cat_bits(vals: &[usize], sizes: &[usize]) -> u8 {
	let mut r = 0usize;
	for (val, size) in vals.iter().zip(sizes.iter()) {
		r = (r << size) | val;
	}
	r as u8
}

pub fn bytes(bs: &[u8]) -> String {
	let mut str = String::new();
	for b in bs.iter() {
		str.push_str(&format!("{:02x}", b));
	}
	str
}

pub const P_LOCK: u8 = 0xF0;
pub const P_REP: u8 = 0xF3;
pub const P_REPNE: u8 = 0xF2;
pub const P_OP_SIZE: u8 = 0x66;
pub const P_ADDR_SIZE: u8 = 0x67;
pub const P_SEG_CS: u8 = 0x2E;
pub const P_SEG_ES: u8 = 0x26;
pub const P_SEG_DS: u8 = 0x3E;
pub const P_SEG_SS: u8 = 0x36;
pub const P_SEG_FS: u8 = 0x64;
pub const P_SEG_GS: u8 = 0x65;

pub const ALL_PREFIXES: &'static [u8] = &[P_LOCK, P_REP, P_REPNE,
	P_OP_SIZE, P_ADDR_SIZE,
	P_SEG_CS, P_SEG_DS, P_SEG_ES, P_SEG_SS, P_SEG_FS, P_SEG_GS];

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum OpOption {
	Rm,
	SSE,
	SSEOff,
	Read,
	Write,
	Implicit(usize, Access),
	FixImm(i64),
	FixReg(usize),
	FixRegRex(usize),
	Cr(bool),
	Prefix(u8),
	OpSizePostfix,
	Imm,
	Addr,
	Reg,
	Disp,
	NoMem,
	Mem(Option<usize>),
	RmOpcode(usize),
	OpSizeLimit32,
	OpSize(Size),
	OpSizeDef,
	ImmSize(Size),
}

use self::OpOption::*;

#[derive(Clone)]
struct State {
	def_op_size: Size,
	operand_size: Size,
	prefix_whitelist: Vec<u8>,
	matched_prefixes: Vec<u8>,
	operands: Vec<(Operand, Size)>,
	imm_size: Size,
	unknown_mem: bool,
	branch: bool,
	no_mem: bool,
}

pub fn list_insts(ops: &mut Vec<Inst>, verify: bool) {
	let opts = |options: &[OpOption], inst: &mut Inst, def_op_size: Size| {
		let mut op_size = SOpSize;
		let mut imm_size = SImmSize;
		let mut regs = Regs::GP;
		let mut access = Access::Write;
		let mut to_read = false;

		for opt in options.iter() {
			if inst.operands.len() >= 1 && !to_read {
				to_read = true;
				access = Access::Read;
			}

			//debug!("Appling option {:?}, opsize = {:?}\n", opt, op_size);
			match *opt {
				ImmSize(size) => {
					imm_size = size;
				}
				OpSize(size) => {
					op_size = size;
				}
				OpSizeDef => {
					op_size = def_op_size;
				}
				OpSizePostfix => {
					inst.op_size_postfix = true;
				}
				Implicit(r, access) => {
					inst.accesses.push((r, access));
				}
				Prefix(p) => {
					inst.prefix_whitelist.push(p);
				}
				Read => {
					access = Access::Read;
				}
				Write => {
					access = Access::Write;
				}
				SSEOff => {
					op_size = op_size;
					regs = Regs::GP;
				}
				OpSizeLimit32 => {
					op_size = if op_size == S64 { S32 } else { op_size };
				}
				SSE => {
					op_size = S128;
					regs = Regs::SSE;
				}
				NoMem => {
					inst.no_mem = true;
				}
				FixRegRex(reg) => {
					inst.operands.push((Operand::FixRegRex(reg, regs), op_size, access));
				}
				FixReg(reg) => {
					inst.operands.push((Operand::FixReg(reg, regs), op_size, access));
				}
				FixImm(imm) => {
					inst.operands.push((Operand::FixImm(imm, Lit1), Lit1, Access::Read));
				}
				Addr => {
					inst.operands.push((Operand::Addr, op_size, access));
				}
				Imm => {
					inst.operands.push((Operand::Imm(imm_size), op_size, Access::Read));
				}
				Disp => {
					inst.operands.push((Operand::Disp(imm_size), S64, Access::Read));
				}
				Rm => {
					inst.operands.push((Operand::Rm(regs), op_size, access));
				}
				Reg => {
					inst.operands.push((Operand::Reg(regs), op_size, access));
				}
				RmOpcode(opcode_ext) => {
					inst.opcode = Some(opcode_ext);
					inst.operands.push((Operand::RmOpcode(opcode_ext), op_size, access));
				}
				Mem(opcode_ext) => {
					inst.opcode = opcode_ext;
					inst.operands.push((Operand::Mem(opcode_ext), op_size, access));
				}
				_ => panic!("unhandled {:?}", opt)
			};
		}
		inst.operand_size = op_size;
	};

	let do_op = |full_code: &[u8], name: &str, options: &[OpOption], def_op_size: Size, ops: &mut Vec<Inst>| {
		let mut prefix_len = 0;
		while ALL_PREFIXES.contains(&full_code[prefix_len]) {
			prefix_len += 1;
		}
		let code_prefixes = &full_code[0..prefix_len];
		let code = &full_code[prefix_len..];

		let mut inst = Inst {
			prefix_bytes: code_prefixes.to_vec(),
			bytes: code.to_vec(),
			opcode: None,
			prefix_whitelist: vec![],
			operands: Vec::new(),
			decoded_operands: Vec::new(),
			op_size_postfix: false,
			accesses: Vec::new(),
			operand_size: SOpSize,
			no_mem: false,
			desc: "".to_string(),
			name: name.to_string(),
			len: 0,
		};

		opts(options, &mut inst, def_op_size);

		ops.push(inst);
	};

	macro_rules! op {
		($code:expr, $name:expr, $opts:expr) => ({
			do_op(&$code, $name, &$opts, SOpSize, ops);
		})
	}

	let os = Prefix(P_OP_SIZE);

	macro_rules! pair {
		($code:expr, $name:expr, $opts:expr) => ({
			let mut o = Vec::new();
			o.push(OpSize(S8));
			o.push(ImmSize(S8));
			o.extend($opts.iter().cloned());
			o.retain(|p| *p != os);
			let mut c = Vec::new();
			c.extend(&$code);
			*c.last_mut().unwrap() += 1;
			do_op(&$code, $name, &o, S8, ops);
			do_op(&c[..], $name, &$opts, SOpSize, ops);
		})
	}

	for (arith_opcode, instr) in ["add", "or", "adc", "sbb", "and", "sub", "xor", "cmp"].iter().enumerate() {
		let o = if *instr == "cmp" {
			Read
		} else {
			Prefix(P_LOCK)
		};

		for (format_num, format) in [[o, os, Rm, Reg].as_ref(), [o, os, Reg, Rm].as_ref(), [o, os, FixReg(0), Imm].as_ref()].iter().enumerate() {
			let opcode = cat_bits(&[arith_opcode, format_num, 0], &[5, 2, 1]);
			let mut f = format.to_vec();
			pair!([opcode], instr, f[..])
		}

		pair!([0x80], instr, [o, os, RmOpcode(arith_opcode), Imm]);
		op!([0x83], instr, [o, os, RmOpcode(arith_opcode), ImmSize(S8), Imm]);
	}

	pair!([0xfe], "inc", [Prefix(P_LOCK), RmOpcode(0)]);
	pair!([0xfe], "dec", [Prefix(P_LOCK), RmOpcode(1)]);

	for &(instr, opcode) in &[("rol", 0), ("ror", 1), ("rcl", 2), ("rcr", 3), ("shl", 4), ("shr", 5), ("sar", 7)] {
		pair!([0xc0], instr, [RmOpcode(opcode), OpSize(S8), ImmSize(S8), Imm]);
		pair!([0xd0], instr, [RmOpcode(opcode), FixImm(1)]);
		pair!([0xd2], instr, [RmOpcode(opcode), OpSize(S8), FixReg(1)]);
	}

	let cond_codes = ["o", "no", "b", "ae", "e", "ne", "be", "a", "s", "ns", "p", "np", "l", "ge", "le", "g"];

	for (cond_num, cond_name) in cond_codes.iter().enumerate() {
		op!([0x70 + cond_num as u8], &format!("j{}", cond_name), [ImmSize(S8), Disp]);
		op!([0x0f, 0x80 + cond_num as u8], &format!("j{}", cond_name), [ImmSize(S32), Disp]);
		op!([0x0f, 0x40 + cond_num as u8], &format!("cmov{}", cond_name), [Reg, Rm]);
		op!([0x0f, 0x90 + cond_num as u8], &format!("set{}", cond_name), [OpSize(S8), Rm]);
	}

	pair!([0xa8], "test", [os, Read, FixReg(0), Imm]);
	pair!([0x84], "test", [os, Read, Rm, Reg]);
	pair!([0xf6], "test", [os, Read, RmOpcode(0), Imm]);

	for &(instr, opcode) in &[("not", 2), ("neg", 3), ("mul", 4), ("imul", 5), ("div", 6), ("idiv", 7)] {
		let mut f8 = vec![OpSize(S8), ImmSize(S8), RmOpcode(opcode)];
		let mut f = vec![RmOpcode(opcode)];
		if opcode >= 4 {
			f.insert(0, Read);
			f8.insert(0, Read);
			f8.push(Implicit(0, Access::Write));
			f.push(Implicit(0, Access::Write));
			f.push(Implicit(2, Access::Write));
		}
		if instr == "not" || instr == "neg" {
			f8.push(Prefix(P_LOCK));
			f.push(Prefix(P_LOCK));
		}
		op!([0xf6], instr, f8[..]);
		op!([0xf7], instr, f[..]);
	}

	let nop_prefixes: Vec<OpOption> = ALL_PREFIXES.iter().filter(|&p| *p != P_LOCK).map(|v| Prefix(*v)).collect();

	let mut opts = nop_prefixes.clone();
	opts.extend([Read, RmOpcode(0)].iter().cloned());
	op!([0x0f, 0x1f], "nop", opts[..]);

	op!([0xeb], "jmp", [ImmSize(S8), Disp]);
	op!([0xe9], "jmp", [ImmSize(S32), Disp]);

	if !verify {
		op!([0xff], "jmp", [RmOpcode(4)]);
	}

	op!([0xe8], "call", [ImmSize(S32), Disp]);
	op!([0xff], "call", [RmOpcode(2)]);

	for reg in 0..8 {
		op!([0x50 + reg], "push", [OpSize(S64), FixRegRex(reg as usize)]);
		op!([0x58 + reg], "pop", [OpSize(S64), FixRegRex(reg as usize)]);
	}

	pair!([0x86], "xchg", [Rm, Write, Reg]);

	pair!([0x88], "mov", [os, Rm, Reg]);
	pair!([0x8a], "mov", [os, Reg, Rm]);
	pair!([0xc6], "mov", [os, RmOpcode(0), Imm]);
	pair!([0xa0], "mov", [FixReg(0), Addr]);
	pair!([0xa2], "mov", [Addr, FixReg(0)]);

	for reg in 0..8 {
		op!([0xb0 + reg], "mov", [OpSize(S8), FixRegRex(reg as usize), ImmSize(S8), Imm]);
		op!([0xb8 + reg], "mov", [os, FixRegRex(reg as usize), ImmSize(SOpSize), Imm]);
	}

	op!([0x0f, 0xa3], "bt", [Rm, Reg]);
	op!([0x0f, 0xab], "bts", [Prefix(P_LOCK), Rm, Reg]);
	op!([0x0f, 0xb3], "btr", [Prefix(P_LOCK), Rm, Reg]);
	op!([0x0f, 0xbb], "btc", [Prefix(P_LOCK), Rm, Reg]);

	op!([0x0f, 0xba], "bt", [RmOpcode(4), ImmSize(S8), Imm]);
	op!([0x0f, 0xba], "bts", [Prefix(P_LOCK), RmOpcode(5), ImmSize(S8), Imm]);
	op!([0x0f, 0xba], "btr", [Prefix(P_LOCK), RmOpcode(6), ImmSize(S8), Imm]);
	op!([0x0f, 0xba], "btc", [Prefix(P_LOCK), RmOpcode(7), ImmSize(S8), Imm]);

	op!([0x0f, 0xaf], "imul", [Reg, Rm]);
	op!([0x69], "imul", [Reg, Rm, Imm]);
	op!([0x6b], "imul", [Reg, Rm, ImmSize(S8), Imm]);

	op!([0xf3, 0x90], "pause", []);

	for reg in 0..8 {
		if reg == 0 {
			op!([0x90], "nop", nop_prefixes[..]) // Check which prefixes are useful here
		} else {
			op!([0x90 + reg as u8], "xchg", [FixReg(0), Write, FixRegRex(reg)])
		}
	}

	op!([0x8d], "lea", [NoMem, Reg, Mem(None)]);

	if !verify {
		op!([0xc3], "ret", []);
	}

	op!([0x0f, 0xb6], "movzx", [Reg, OpSize(S8), Rm]);
	op!([0x0f, 0xb7], "movzx", [Reg, OpSize(S16), Rm]); // TODO: Incorrect Opsize on first operand
	op!([0x0f, 0xbe], "movsx", [Reg, OpSize(S8), Rm]);
	op!([0x0f, 0xbf], "movsx", [Reg, OpSize(S16), Rm]);

	op!([0x63], "movsxd", [OpSize(S64), Reg, OpSize(S32), Rm]); // Require rex_w here

	op!([0x66, 0x98], "cbw", [Implicit(0, Access::Write)]);
	op!([0x98], "cwde", [Implicit(0, Access::Write)]); // Named 'cdqe' with rex_w

	op!([0x66, 0x99], "cwd", [Implicit(2, Access::Write)]);
	op!([0x99], "cdq", [Implicit(2, Access::Write)]); // Named 'cqo' with rex_w

	op!([0xcc], "int3", []);

	op!([0x0f, 0x0b], "ud2", []);

	pair!([0x0f, 0xb0], "cmpxchg", [Prefix(P_LOCK), Rm, Reg]);
	pair!([0x0f, 0xc0], "xadd", [Prefix(P_LOCK), Rm, Reg]);

	op!([0x0f, 0xae, 0xf0], "mfence", []);

	if !verify {
		pair!([0xa4], "movs", [OpSizePostfix, Prefix(P_REP)]);
	}

	// SSE

	op!([0x66, 0x0f, 0x5c], "subpd", [SSE, Reg, Rm]);
	op!([0x0f, 0x5c], "subps", [SSE, Reg, Rm]);

	op!([0xf2, 0x0f, 0x5c], "subsd", [SSE, Reg, OpSize(S64), Rm]);
	op!([0xf3, 0x0f, 0x5c], "subss", [SSE, Reg, OpSize(S32), Rm]);

	op!([0x66, 0x0f, 0x5e], "divpd", [SSE, Reg, Rm]);
	op!([0x0f, 0x5e], "divps", [SSE, Reg, Rm]);

	op!([0xf2, 0x0f, 0x5e], "divsd", [SSE, Reg, OpSize(S64), Rm]);
	op!([0xf3, 0x0f, 0x5e], "divss", [SSE, Reg, OpSize(S32), Rm]);

	op!([0x66, 0x0f, 0x59], "mulpd", [SSE, Reg, Rm]);
	op!([0x0f, 0x59], "mulps", [SSE, Reg, Rm]);

	op!([0xf2, 0x0f, 0x59], "mulsd", [SSE, Reg, OpSize(S64), Rm]);
	op!([0xf3, 0x0f, 0x59], "mulss", [SSE, Reg, OpSize(S32), Rm]);

	op!([0x66, 0x0f, 0x58], "addpd", [SSE, Reg, Rm]);
	op!([0x0f, 0x58], "addps", [SSE, Reg, Rm]);

	op!([0xf2, 0x0f, 0x58], "addsd", [SSE, Reg, OpSize(S64), Rm]);
	op!([0xf3, 0x0f, 0x58], "addss", [SSE, Reg, OpSize(S32), Rm]);

	op!([0xf2, 0x0f, 0x10], "movsd", [SSE, Reg, OpSize(S64), Rm]);
	op!([0xf2, 0x0f, 0x11], "movsd", [SSE, OpSize(S64), Rm, OpSize(S128), Reg]); 

	op!([0x66, 0x0f, 0x28], "movapd", [SSE, Reg, Rm]);
	op!([0x66, 0x0f, 0x29], "movapd", [SSE, Rm, Reg]);

	op!([0x0f, 0x28], "movaps", [SSE, Reg, Rm]);
	op!([0x0f, 0x29], "movaps", [SSE, Rm, Reg]);

	op!([0x66, 0x0f, 0x2e], "ucomisd", [SSE, OpSize(S64), Read, Reg, Rm]);
	op!([0x0f, 0x2e], "ucomiss", [SSE, OpSize(S32), Read, Reg, Rm]);

	op!([0x66, 0x0f, 0x54], "andpd", [SSE, Reg, Rm]);
	op!([0x0f, 0x54], "andps", [SSE, Reg, Rm]);

	op!([0xf2, 0x0f, 0x2c], "cvttsd2si", [OpSize(SRexSize), Reg, SSE, OpSize(S64), Rm]);
	op!([0xf3, 0x0f, 0x2c], "cvttss2si", [OpSize(SRexSize), Reg, SSE, OpSize(SRexSize), Rm]);

	op!([0xf2, 0x0f, 0x2a], "cvtsi2sd", [SSE, Reg, OpSize(SRexSize), SSEOff, Rm]);
	op!([0xf3, 0x0f, 0x2a], "cvtsi2ss", [SSE, Reg, OpSize(SRexSize), SSEOff, Rm]);

	op!([0xf3, 0x0f, 0x7e], "movq", [SSE, OpSize(S64), Reg, Rm]);
	op!([0x66, 0x0f, 0xd6], "movq", [SSE, OpSize(S64), Rm, Reg]);

	op!([0x66, 0x0f, 0x6e], "mov", [OpSizePostfix, SSE, Reg, OpSize(SRexSize), SSEOff, Rm]);
	op!([0x66, 0x0f, 0x7e], "mov", [OpSizePostfix, OpSize(SRexSize), Rm, SSE, Reg, OpSize(SRexSize)]);

	op!([0x66, 0x0f, 0x38, 00], "pshufb", [SSE, Reg, Rm]);
	op!([0x66, 0x0f, 0x70], "pshufd", [SSE, Reg, Rm, ImmSize(S8), Imm]);

	op!([0x66, 0x0f, 0x6c], "punpcklqdq", [SSE, Reg, Rm]);
	op!([0x66, 0x0f, 0x6d], "punpckhqdq", [SSE, Reg, Rm]);

	op!([0x66, 0x0f, 0x6f], "movdqa", [SSE, Reg, Rm]);
	op!([0x66, 0x0f, 0x7f], "movdqa", [SSE, Rm, Reg]);

	op!([0xf3, 0x0f, 0x6f], "movdqu", [SSE, Reg, Rm]);
	op!([0xf3, 0x0f, 0x7f], "movdqu", [SSE, Rm, Reg]);

	op!([0x66, 0x0f, 0x10], "movupd", [SSE, Reg, Rm]);
	op!([0x66, 0x0f, 0x11], "movupd", [SSE, Rm, Reg]);

	op!([0x0f, 0x10], "movups", [SSE, Reg, Rm]);
	op!([0x0f, 0x11], "movups", [SSE, Rm, Reg]);

	op!([0x66, 0x0f, 0x57], "xorpd", [SSE, Reg, Rm]);
	op!([0x0f, 0x57], "xorps", [SSE, Reg, Rm]);

	// Used for syscalls for now
	op!([0xcd], "int", [OpSize(S8), ImmSize(S8), Imm]);

	// System Instructions
	if true {
		return;
	}

	op!([0x0f, 0x00], "ltr", [NoMem, OpSize(S16), RmOpcode(3)]);
	op!([0x0f, 0x01], "lgdt", [NoMem, Mem(Some(2))]);
	op!([0x0f, 0x01], "lidt", [NoMem, Mem(Some(3))]);
	op!([0x0f, 0x01], "invlpg", [NoMem, Mem(Some(7))]);

	op!([0x0f, 0x20], "mov", [Cr(true)]);
	op!([0x0f, 0x22], "mov", [Cr(false)]);

	op!([0x0f, 0x32], "rdmsr", []);
	op!([0x0f, 0x30], "wrmsr", []);

	op!([0xfa], "cli", []);
	op!([0xfb], "sti", []);

	op!([0xf4], "hlt", []);

	pair!([0xe4], "in", [OpSizeLimit32, FixReg(0), OpSize(S8), ImmSize(S8), Imm]);
	pair!([0xec], "in", [OpSizeLimit32, FixReg(0), OpSize(S16), FixReg(2)]);

	pair!([0xe6], "out", [OpSize(S8), ImmSize(S8), Imm, OpSizeDef, OpSizeLimit32, FixReg(0)]);
	pair!([0xee], "out", [OpSize(S16), FixReg(2), OpSizeDef, OpSizeLimit32, FixReg(0)]);
}