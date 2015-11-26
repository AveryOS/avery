use decoder::Cursor;
use std::cell::RefCell;

//trace_macros!(true);

enum Registers {
	RAX, RCX, RDX, RBX, RSP, RBP, RSI, RDI,
		  R8, R9, R10, R11, R12, R13, R14, R15
}

fn ext_bit(b: usize, i: usize, t: usize) -> usize {
	((b >> i) & 1) << t
}

#[derive(Clone)]
pub struct Instruction {
	pub desc: String,
	pub terminating: bool,
	pub ops: Vec<(Operand, Size)>,
	pub branch: bool,
}

#[derive(Clone)]
struct IndirectAccess {
	base: Option<usize>,
	index: Option<usize>,
	scale: usize,
	offset: i64,
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum Size {
	S8,
	S16,
	S32,
	S64,
}
use self::Size::*;

#[derive(Clone)]
pub enum Operand {
	Direct(usize),
	Indirect(IndirectAccess),
	Imm((i64, Size)),
}

const P_LOCK: u8 = 0xF0;
const P_REP: u8 = 0xF3;
const P_REPNE: u8 = 0xF2;
const P_OP_SIZE: u8 = 0x66;
const P_ADDR_SIZE: u8 = 0x67;
const P_SEG_CS: u8 = 0x2E;
const P_SEG_ES: u8 = 0x26;
const P_SEG_DS: u8 = 0x3E;
const P_SEG_SS: u8 = 0x36;
const P_SEG_FS: u8 = 0x64;
const P_SEG_GS: u8 = 0x65;

pub const ALL_PREFIXES: &'static [u8] = &[P_LOCK, P_REP, P_REPNE,
	P_OP_SIZE, P_ADDR_SIZE,
	P_SEG_CS, P_SEG_DS, P_SEG_ES, P_SEG_SS, P_SEG_FS, P_SEG_GS];

#[derive(Clone)]
enum OpOption {
	Rm,
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
	Branch,
	UnknownMem,
	NoMem,
	MatchPrefix(Vec<u8>),
	Mem(Option<usize>),
	RmOpcode(usize),
	OpSize(Size),
	OpSizeDef,
	ImmSize(Size),
	Term,
}

use self::OpOption::*;

fn cat_bits(vals: &[usize], sizes: &[usize]) -> u8 {
	let mut r = 0usize;
	for (val, size) in vals.iter().zip(sizes.iter()) {
		r = (r << size) | val;
	}
	r as u8
}

fn sign_hex(i: i64, plus: bool) -> String {
	if i < 0 {
		format!("-{:#x}", -i)
	} else {
		format!("{}{:#x}", if plus { "+" } else { "" }, i)
	}
}

#[derive(Clone)]
struct State<'s> {
	cursor: Cursor<'s>,
	def_op_size: Size,
	operand_size: Size,
	prefix_whitelist: Vec<u8>,
	matched_prefixes: Vec<u8>,
	operands: Vec<(Operand, Size)>,
	imm_size: Size,
	terminating: bool,
	modrm_cache: Option<(Operand, usize)>,
	op_size_postfix: bool,
	unknown_mem: bool,
	branch: bool,
	no_mem: bool,
}

const REGS64: &'static [&'static str] = &["rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi",
	  "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15",
	  "rip", "cr0", "cr1", "cr2", "cr3", "cr4", "cr5", "cr6", "cr7", "cr8", // Extras
	  "cr9", "cr10", "cr11", "cr12", "cr13", "cr14", "cr15"]; 
const REGS32: &'static [&'static str] = &["eax", "ecx", "edx", "ebx", "esp", "ebp", "esi", "edi",
		  "r8d", "r9d", "r10d", "r11d", "r12d", "r13d", "r14d", "r15d"];
const REGS16: &'static [&'static str] = &["ax", "cx", "dx", "bx", "sp", "bp", "si", "di",
		  "r8w", "r9w", "r10w", "r11w", "r12w", "r13w", "r14w", "r15w"];
const REGS8 : &'static [&'static str] = &["al", "cl", "dl", "bl", "spl", "bpl", "sil", "dil",
				  "r8b", "r9b", "r10b", "r11b", "r12b", "r13b", "r14b", "r15b"];
const REGS8_NOREX : &'static [&'static str] = &["al", "cl", "dl", "bl", "ah", "ch", "dh", "bh"];

const LOCK_WHITELIST : &'static [&'static str] = &[
    "adc", "add", "and", "btc", "btr", "bts",
    "cmpxchg", "cmpxchg8b", "cmpxchg16b",
    "dec", "inc",
    "neg", "not", "or", "sbb", "sub",
    "xadd", "xchg", "xor"];

fn operand_ptr(size_known: bool, op_size: Size) -> &'static str {
	if size_known {
		return "";
	};
	match op_size {
		S64 => "qword ",
		S32 => "dword ",
		S16 => "word ",
		S8 => "byte ",
	}
}

fn reg_name(r: usize, op_size: Size, rex: bool) -> &'static str  {
	match op_size {
		S64 => REGS64[r],
		S32 => REGS32[r],
		S16 => REGS16[r],
		S8 => if rex { REGS8[r] } else { REGS8_NOREX[r] },
	}
}

fn read_imm(s: &RefCell<State>, size: Size) -> i64 {
	let mut c = &mut s.borrow_mut().cursor;
	match size {
		S8 => {
			c.next() as i8 as i64
		}
		S16 => {
			let mut v = c.next() as u32;
			v |= (c.next() as u32) << 8;
			v as i32 as i64
		}
		S32 => {
			let mut v = c.next() as u32;
			v |= (c.next() as u32) << 8;
			v |= (c.next() as u32) << 16;
			v |= (c.next() as u32) << 24;
			v as i32 as i64
		}
		S64 => {
			let mut v = c.next() as u64;
			v |= (c.next() as u64) << 8;
			v |= (c.next() as u64) << 16;
			v |= (c.next() as u64) << 24;
			v |= (c.next() as u64) << 32;
			v |= (c.next() as u64) << 40;
			v |= (c.next() as u64) << 48;
			v |= (c.next() as u64) << 56;
			v as i64
		}
	}
}

pub fn parse(in_cursor: &mut Cursor, rex: Option<u8>, prefixes: &[u8], disp_off: u64) -> Option<Instruction> {
	let rex = rex.unwrap_or(0);
	let rex_w = ext_bit(rex as usize, 3, 0) != 0;
	let rex_b = ext_bit(rex as usize, 0, 0) != 0;
	let operand_size_override = prefixes.contains(&P_OP_SIZE);
	let fs_override = prefixes.contains(&P_SEG_FS);
	let gs_override = prefixes.contains(&P_SEG_GS);

	if gs_override && fs_override {
		return None;
	}

	let op_size = if rex_w { S64 } else { if operand_size_override { S16 } else { S32 } };
	let state: RefCell<State> = RefCell::new(State {
		cursor: in_cursor.clone(),
		terminating: false,
		matched_prefixes: Vec::new(),
		prefix_whitelist: vec![P_LOCK], // lock has it's own whitelist
		operands: Vec::new(),
		def_op_size: op_size, // overriden by pair!
		operand_size: op_size,
		imm_size: if op_size == S16 { S16 } else { S32 },
		modrm_cache: None,
		op_size_postfix: false,
		unknown_mem: false,
		branch: false,
		no_mem: false,
	});

	let sr = || state.borrow();
	let s = || state.borrow_mut();

	let has_prefix = |p: u8| {
		if sr().matched_prefixes.contains(&p) {
			false
		} else {
			prefixes.contains(&p)
		}
	};

	let print_op = |i: usize| {
		let op = sr().operands[i].clone();
		let known_size = sr().operands.iter().any(|&(ref o, s)| match o {
			&Operand::Direct(..) => s == op.1,
			_ => false,
		});

		match op.0 {
			Operand::Direct(reg) => format!("{}", reg_name(reg, op.1, rex != 0)),
			Operand::Indirect(indir) => {
				let ptr = operand_ptr(!sr().unknown_mem && (known_size || sr().no_mem), op.1);

				let scale = if indir.scale == 1 {
					"".to_string()
				} else {
					format!("*{}", indir.scale)
				};

				let segment = if fs_override {
					"fs:"
				} else if gs_override {
					"gs:"
				} else {
					""
				};

				let name = match &(indir.base, indir.index) {
					&(Some(base), Some(index)) => format!("{}+{}{}", REGS64[base], REGS64[index], scale),
					&(None, Some(index)) => format!("{}{}", REGS64[index], scale),
					&(Some(base), None) => format!("{}", REGS64[base]),
					&(None, None) => return format!("{}[{}{:#x}]", ptr, segment, indir.offset as i32),
				};

				if indir.offset != 0 {
					format!("{}[{}{}{}]", ptr, segment, name, sign_hex(indir.offset, true))
				} else {
					format!("{}[{}{}]", ptr, segment, name)
				}
			}
			Operand::Imm((im, size)) => match op.1 {
				S8 => format!("{:#x}", im as i8),
				S16 => format!("{:#x}", im as i16),
				S32 => format!("{:#x}", im as i32),
				S64 => format!("{:#x}", im),
			}
		}
	};

	let modrm = || {
		if !sr().modrm_cache.is_some() {
			let modrm = s().cursor.next() as usize;
			let mode = modrm >> 6;
			let reg = ((modrm >> 3) & 7) | ext_bit(rex as usize, 2, 3);
			let rm = modrm & 7 | ext_bit(rex as usize, 0, 3);

			//println!("mode:{} reg:{} rm: {}", mode ,reg ,rm);

			let mut name = if mode != 3 && rm & 7 == 4 {
				// Parse SIB byte

				let sib = s().cursor.next() as usize;
				let base = sib & 7 | ext_bit(rex as usize, 0, 3);
				let index = ((sib >> 3) & 7) | ext_bit(rex as usize, 1, 3);
				let scale = sib >> 6;

				let reg_index = if index == 4 {
					None
				} else {
					Some(index)
				};
				let (reg_base, off) = if mode == 0 && base & 7 == 5 {
					(None, read_imm(&state, S32))
				} else {
					(Some(base), 0)
				};

				IndirectAccess {
					base: reg_base,
					index: reg_index,
					scale: 1 << scale,
					offset: off,
				}
			} else {
				if mode == 0 && rm & 7 == 5 { // RIP relative
					let off = read_imm(&state, S32);

					IndirectAccess {
						base: Some(16), // RIP
						index: None,
						scale: 0,
						offset: off,
					}
				} else {
					IndirectAccess {
						base: Some(rm),
						index: None,
						scale: 0,
						offset: 0,
					}
				}
			};

			let off = match mode {
				0 | 3 => name.offset,
				1 => read_imm(&state, S8),
				2 => read_imm(&state, S32),
				_ => panic!(),
			};

			let indir = if mode == 3 {
				Operand::Direct(rm)
			} else {
				name.offset = off;
				Operand::Indirect(name)
			};

			s().modrm_cache = Some((indir, reg));
		}
		sr().modrm_cache.as_ref().unwrap().clone()
	};

	let opts = |options: &[OpOption]| {
		let mut allowed = true;
		for opt in options.iter() {
			let opsize = sr().operand_size;
			let l_allowed = match *opt {
				ImmSize(size) => {
					s().imm_size = size;
					true
				}
				OpSize(size) => {
					s().operand_size = size;
					true
				}
				OpSizeDef => {
					let d = sr().def_op_size;
					s().operand_size = d;
					true
				}
				OpSizePostfix => {
					s().op_size_postfix = true;
					true
				}
				MatchPrefix(ref bs) => {
					if prefixes.ends_with(&bs) {
						for p in bs.iter() {
							s().prefix_whitelist.push(*p);
							s().matched_prefixes.push(*p);
						}
						true
					} else {
						false
					}
				}
				Term => {
					s().terminating = true;
					true
				}
				Prefix(p) => {
					s().prefix_whitelist.push(p);
					true
				}
				NoMem => {
					s().no_mem = true;
					true
				}
				UnknownMem => {
					s().unknown_mem = true;
					true
				}
				FixRegRex(mut reg) => {
					if rex_b {
						reg += 8;
					}
					s().operands.push((Operand::Direct(reg), opsize));
					true
				}
				FixReg(reg) => {
					s().operands.push((Operand::Direct(reg), opsize));
					true
				}
				FixImm(imm) => {
					s().operands.push((Operand::Imm((imm, S8)), opsize));
					true
				}
				Cr(read) => {
					let modrm = s().cursor.next() as usize;
					let reg = ((modrm >> 3) & 7) | ext_bit(rex as usize, 2, 3);
					let rm = modrm & 7 | ext_bit(rex as usize, 0, 3);
					let (src, dst) = if read {
						(reg + 17, rm)
					} else {
						(rm, reg + 17)
					};
					s().operands.push((Operand::Direct(dst), S64));
					s().operands.push((Operand::Direct(src), S64));
					true
				}
				Addr => {
					let a = IndirectAccess {
						base: None,
						index: None,
						scale: 0,
						offset: read_imm(&state, S64),
					};
					s().operands.push((Operand::Indirect(a), opsize));
					true
				}
				Imm => {
					let imm_size = sr().imm_size;
					let imm = read_imm(&state, imm_size);
					s().operands.push((Operand::Imm((imm, imm_size)), opsize));
					true
				}
				Branch => {
					s().branch = true;
					true
				}
				Disp => {
					let imm_size = sr().imm_size;
					let off = read_imm(&state, imm_size).wrapping_add(disp_off as i64).wrapping_add(s().cursor.offset as u64 as i64);
					s().branch = true;
					s().operands.push((Operand::Imm((off, imm_size)), S64));
					true
				}
				Mem(opcode_ext) => {
					let (indir, reg) = modrm();
					if let Some(opcode_ext) = opcode_ext {
						if reg & 7 != opcode_ext {
							return false;
						}
					}
					match indir {
						Operand::Indirect(..) => {
							s().operands.push((indir, opsize));
							true
						}
						_ => false,
					}
				}
				Rm => {
					let (indir, _) = modrm();
					s().operands.push((indir, opsize));
					true
				}
				Reg => {
					let (_, reg) = modrm();
					s().operands.push((Operand::Direct(reg), opsize));
					true
				}
				RmOpcode(opcode_ext) => {
					let (indir, reg) = modrm();
					s().operands.push((indir.clone(), opsize));
					//println!("RmOpcode {} {} {}", reg & 7, opcode_ext, print_op(sr().operands.len() - 1));
					s().operands.pop();
					if reg & 7 != opcode_ext {
						false
					} else {
						s().operands.push((indir, opsize));
						true
					}
				}
			};

			if !l_allowed {
				allowed = false;
				break;
			}
		}
		allowed
	};

	let valid_state = |name: &str| {
		let mut prefix = String::new();

		if has_prefix(P_LOCK) {
			if !LOCK_WHITELIST.contains(&name) {
				return None;
			} else {
				prefix.push_str("lock ");
			}
		}

		if has_prefix(P_REP) {
			prefix.push_str("rep ");
		}

		if !sr().no_mem {
			let len = sr().operands.len();
			for i in 0..len {
				let op = sr().operands[i].0.clone();
				match op {
					Operand::Direct(..) | Operand::Indirect(..) => s().prefix_whitelist.push(P_OP_SIZE),
					_ => ()
				}
			}
		}

		if prefixes.iter().all(|p| sr().prefix_whitelist.contains(p)) {
			Some(prefix)
		} else {
			None
		}
	};

	let do_op = |code: &[u8], name: &str, options: &[OpOption]| -> Option<Option<Instruction>> {
		let temp_state = sr().clone();
		if sr().cursor.remaining().starts_with(code) {
			s().cursor.offset += code.len();
			if opts(options) {
				let op_size_postfix = sr().op_size_postfix;
				let iname = if op_size_postfix {
					s().prefix_whitelist.push(P_OP_SIZE);
					name.to_string() + match sr().operand_size {
						S64 => "q",
						S32 => "d",
						S16 => "w",
						S8 => "b",
					}
				} else {
					name.to_string()
				};
				let prefix = match valid_state(name) {
					Some(p) => p,
					None => {
						*s() = temp_state;
						return Some(None)
					}
				};
				let ops = sr().operands.iter().enumerate().map(|(i, _)| print_op(i)).collect::<Vec<String>>().join(", ");
				return Some(Some(Instruction {
					desc: format!("{}{}{}{}", prefix, iname, if sr().operands.is_empty() { "" } else { " " }, ops),
					ops: sr().operands.clone(),
					branch: sr().branch,
					terminating: sr().terminating,
				}));
			} else {
				*s() = temp_state;
			}
		}

		None
	};

	macro_rules! op {
		($code:expr, $name:expr, $opts:expr) => ({
			match do_op(&$code, $name, &$opts) {
				Some(v) => {
					*in_cursor = sr().cursor.clone();
					return v;
				}
				None => (),
			}
		})
	}

	macro_rules! pair {
		($code:expr, $name:expr, $opts:expr) => ({
			let mut o = Vec::new();
			o.push(OpSize(S8));
			o.push(ImmSize(S8));
			o.extend($opts.iter().cloned());
			s().def_op_size = S8;
			let mut c = Vec::new();
			c.extend(&$code);
			*c.last_mut().unwrap() += 1;
			op!($code, $name, *o);
			s().def_op_size = op_size;
			op!(c[..], $name, $opts);
		})
	}

	let disp_size = ImmSize(if operand_size_override { S16 } else { S32 });
	let wide_op = OpSize(if operand_size_override { S16 } else { S64 });

	for (arith_opcode, instr) in ["add", "or", "adc", "sbb", "and", "sub", "xor", "cmp"].iter().enumerate() {
		for (format_num, format) in [[Rm, Reg].as_ref(), [Reg, Rm].as_ref(), [FixReg(0), Imm].as_ref()].iter().enumerate() {
			let opcode = cat_bits(&[arith_opcode, format_num, 0], &[5, 2, 1]);
			pair!([opcode], instr, *format)
		}

		pair!([0x80], instr, [RmOpcode(arith_opcode), Imm]);
		op!([0x83], instr, [RmOpcode(arith_opcode), ImmSize(S8), Imm]);
	}

	pair!([0xfe], "inc", [RmOpcode(0)]);
	pair!([0xfe], "dec", [RmOpcode(1)]);

	for &(instr, opcode) in &[("rol", 0), ("ror", 1), ("rcl", 2), ("rcr", 3), ("shl", 4), ("shr", 5), ("sar", 7)] {
		pair!([0xc0], instr, [RmOpcode(opcode), UnknownMem, ImmSize(S8), Imm]);
		pair!([0xd0], instr, [RmOpcode(opcode), UnknownMem, FixImm(1)]);
		pair!([0xd2], instr, [RmOpcode(opcode), UnknownMem, OpSize(S8), FixReg(1)]);
	}

	let cond_codes = ["o", "no", "b", "ae", "z", "nz", "be", "a", "s", "ns", "p", "np", "l", "ge", "le", "g"];

	for (cond_num, cond_name) in cond_codes.iter().enumerate() {
		op!([0x70 + cond_num as u8], &format!("j{}", cond_name), [ImmSize(S8), Disp]);
		op!([0x0f, 0x80 + cond_num as u8], &format!("j{}", cond_name), [disp_size.clone(), Disp]);
		op!([0x0f, 0x40 + cond_num as u8], &format!("cmov{}", cond_name), [Reg, Rm]);
		op!([0x0f, 0x90 + cond_num as u8], &format!("set{}", cond_name), [OpSize(S8),RmOpcode(0)]);
	}

	pair!([0xa8], "test", [FixReg(0), Imm]);
	pair!([0xf6], "test", [RmOpcode(0), Imm]);
	for &(instr, opcode) in &[("not", 2), ("neg", 3), ("mul", 4), ("imul", 5), ("div", 6), ("idiv", 7)] {
		pair!([0xf6], instr, [RmOpcode(opcode)]);
	}

	op!([0x0f, 0xaf], "imul", [Reg, Rm]);

	let nop_prefixes: Vec<OpOption> = ALL_PREFIXES.iter().filter(|&p| *p != P_LOCK).map(|v| Prefix(*v)).collect();

	let mut opts = nop_prefixes.clone();
	opts.extend([RmOpcode(0), NoMem].iter().cloned());
	op!([0x0f, 0x1f], "nop", opts[..]);

	op!([0xeb], "jmp", [ImmSize(S8), Term, Disp]);
	op!([0xe9], "jmp", [Term, disp_size.clone(), Disp]);
	op!([0xff], "jmp", [OpSize(S64), RmOpcode(4), Term, Branch]);

	op!([0xe8], "call", [disp_size.clone(), Disp]);
	op!([0xff], "call", [OpSize(S64), RmOpcode(2), Branch]);

	for reg in 0..8 {
		op!([0x50 + reg], "push", [wide_op.clone(), FixRegRex(reg as usize)]);
		op!([0x58 + reg], "pop", [wide_op.clone(), FixRegRex(reg as usize)]);
	}

	pair!([0x84], "test", [Rm, Reg]);

	op!([0x87, 0xc0], "nop", []); // Really xchg eax, eax which udis displays as nop
	pair!([0x86], "xchg", [Rm, Reg]);

	pair!([0x88], "mov", [Rm, Reg]);
	pair!([0x8a], "mov", [Reg, Rm]);
	pair!([0xc6], "mov", [RmOpcode(0), Imm]);
	pair!([0xa0], "mov", [FixReg(0), Addr]);
	pair!([0xa2], "mov", [Addr, FixReg(0)]);

	for reg in 0..8 {
		op!([0xb0 + reg], "mov", [OpSize(S8), FixRegRex(reg as usize), ImmSize(S8), Imm]);
		op!([0xb8 + reg], "mov", [FixRegRex(reg as usize), ImmSize(op_size), Imm]);
	}

	op!([0x0f, 0xa3], "bt", [Rm, Reg]);
	op!([0x0f, 0xab], "bts", [Rm, Reg]);
	op!([0x0f, 0xb3], "btr", [Rm, Reg]);
	op!([0x0f, 0xbb], "btc", [Rm, Reg]);

	op!([0x0f, 0xba], "bt", [RmOpcode(4), ImmSize(S8), Imm]);
	op!([0x0f, 0xba], "bts", [RmOpcode(5), ImmSize(S8), Imm]);
	op!([0x0f, 0xba], "btr", [RmOpcode(6), ImmSize(S8), Imm]);
	op!([0x0f, 0xba], "btc", [RmOpcode(7), ImmSize(S8), Imm]);

	op!([0x69], "imul", [Reg, Rm, Imm]);
	op!([0x6b], "imul", [Reg, Rm, ImmSize(S8), Imm]);

	op!([0x90], "pause", [MatchPrefix(vec![0xF3])]);

	for reg in 0..8 {
		if reg == 0 && !rex_b && !rex_w {
			op!([0x90], "nop", [])
		} else {
			op!([0x90 + reg as u8], "xchg", [FixRegRex(reg), FixReg(0)])
		}
	}

	op!([0x8d], "lea", [NoMem, Reg, Mem(None)]);

	op!([0xc3], "ret", [Term]);

	op!([0x0f, 0xb6], "movzx", [Reg, OpSize(S8), Rm]);
	op!([0x0f, 0xb7], "movzx", [Reg, OpSize(S16), Rm]);

	op!([0xcc], "int3", []);

	op!([0x0f, 0x0b], "ud2", []);

	pair!([0x0f, 0xb0], "cmpxchg", [Rm, Reg]);
	pair!([0x0f, 0xc0], "xadd", [Rm, Reg]);

	op!([0x0f, 0xae, 0xf0], "mfence", []);

	pair!([0xa4], "movs", [OpSizePostfix, Prefix(P_REP)]);

	// System Instructions

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

	op!([0xcd], "int", [ImmSize(S8), Imm]);

	pair!([0xe4], "in", [FixReg(0), OpSize(S8), ImmSize(S8), Imm]);
	pair!([0xec], "in", [FixReg(0), OpSize(S16), FixReg(2)]);

	pair!([0xe6], "out", [OpSize(S8), ImmSize(S8), Imm, OpSizeDef, FixReg(0)]);
	pair!([0xee], "out", [OpSize(S16), FixReg(2), OpSizeDef, FixReg(0)]);

	None
}