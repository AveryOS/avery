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

#[derive(Clone)]
enum OpOption {
	Rm,
	FixImm(i64),
	FixReg(usize),
	FixRegRex(usize),
	Imm,
	Reg,
	Disp,
	Branch,
	NoMem,
	RmOpcode(usize),
	OpSize(Size),
	ImmSize(Size),
	ImmSizeOp,
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
	operand_size: Size,
	operands: Vec<(Operand, Size)>,
	imm_size: Size,
	terminating: bool,
	modrm_cache: Option<(Operand, usize)>,
	branch: bool,
	no_mem: bool,
}

const REGS64: &'static [&'static str] = &["rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi",
	  "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15"];
const REGS32: &'static [&'static str] = &["eax", "ecx", "edx", "ebx", "esp", "ebp", "esi", "edi",
		  "r8d", "r9d", "r10d", "r11d", "r12d", "r13d", "r14d", "r15d"];
const REGS16: &'static [&'static str] = &["ax", "cx", "dx", "bx", "sp", "bp", "si", "di",
		  "r8w", "r9w", "r10w", "r11w", "r12w", "r13w", "r14w", "r15w"];
const REGS8 : &'static [&'static str] = &["al", "cl", "dl", "bl", "spl", "bpl", "sil", "dil",
				  "r8b", "r9b", "r10b", "r11b", "r12b", "r13b", "r14b", "r15b"];

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

fn reg_name(r: usize, op_size: Size) -> &'static str  {
	match op_size {
		S64 => REGS64[r],
		S32 => REGS32[r],
		S16 => REGS16[r],
		S8 => REGS8[r],
	}
}

pub fn parse(in_cursor: &mut Cursor, rex: Option<u8>, prefixes: &[u8]) -> Option<Instruction> {
	let rex = rex.unwrap_or(0);
	let rex_w = ext_bit(rex as usize, 3, 0) != 0;
	let rex_b = ext_bit(rex as usize, 0, 0) != 0;
	let operand_size_override = prefixes.contains(&0x66);

	let op_size = if rex_w { S64 } else { if operand_size_override { S16 } else { S32 } };
	let state = RefCell::new(State {
		cursor: in_cursor.clone(),
		terminating: false,
		operands: Vec::new(),
		operand_size: op_size,
		imm_size: if op_size == S16 { S16 } else { S32 },
		modrm_cache: None,
		branch: false,
		no_mem: false,
	});

	let sr = || state.borrow();
	let s = || state.borrow_mut();

	let print_op = |i: usize| {
		let op = sr().operands[i].clone();
		let known_size = sr().operands.iter().any(|&(ref o, s)| match o {
			&Operand::Direct(..) => s == op.1,
			_ => false,
		});

		match op.0 {
			Operand::Direct(reg) => format!("{}", reg_name(reg, op.1)),
			Operand::Indirect(indir) => {
				let ptr = operand_ptr(known_size || sr().no_mem, op.1);

				let scale = if indir.scale == 1 {
					"".to_string()
				} else {
					format!("{}*", indir.scale)
				};

				let name = match &(indir.base, indir.index) {
					&(Some(base), Some(index)) => format!("{}+{}{}", REGS64[base], scale, REGS64[index]),
					&(None, Some(index)) => format!("{}{}", scale, REGS64[index]),
					&(Some(base), None) => format!("{}", REGS64[base]),
					&(None, None) => return format!("{}[{:#x}]", ptr, indir.offset),
				};

				if indir.offset != 0 {
					format!("{}[{}{}]", ptr, name, sign_hex(indir.offset, true))
				} else {
					format!("{}[{}]", ptr, name)
				}
			}
			Operand::Imm((im, size)) => match size {
				//S32 => format!("{:#x}", im as i32),
				_ => format!("{:#x}", im),
			}
		}
	};

	let read_imm = |size: Size| {
		match size {
			S8 => {
				s().cursor.next() as i8 as i64
			}
			S16 => {
				let mut v = s().cursor.next() as u32;
				v |= (s().cursor.next() as u32) << 8;
				v as i32 as i64
			}
			S32 => {
				let mut v = s().cursor.next() as u32;
				v |= (s().cursor.next() as u32) << 8;
				v |= (s().cursor.next() as u32) << 16;
				v |= (s().cursor.next() as u32) << 24;
				v as i32 as i64
			}
			S64 => {
				let mut v = s().cursor.next() as u64;
				v |= (s().cursor.next() as u64) << 8;
				v |= (s().cursor.next() as u64) << 16;
				v |= (s().cursor.next() as u64) << 24;
				v |= (s().cursor.next() as u64) << 32;
				v |= (s().cursor.next() as u64) << 40;
				v |= (s().cursor.next() as u64) << 48;
				v |= (s().cursor.next() as u64) << 56;
				v as i64
			}
		}
	};

	let modrm = || {
		if !sr().modrm_cache.is_some() {
			let modrm = s().cursor.next() as usize;
			let mode = modrm >> 6;
			let reg = ((modrm >> 3) & 7) | ext_bit(rex as usize, 2, 3);
			let rm = modrm & 7 | ext_bit(rex as usize, 0, 3);

			let mut name = if mode != 3 && rm & 7 == 4 {
				// Parse SIB byte

				let sib = s().cursor.next() as usize;
				let base = sib & 7 | ext_bit(rex as usize, 0, 3);
				let index = ((sib >> 3) & 7) | ext_bit(rex as usize, 1, 3);
				let scale = sib >> 6;

				let reg_index = if index == 5 {
					None
				} else {
					Some(index)
				};
				let mut reg_base = if mode == 0 && base & 7 == 5 {
					None
				} else {
					Some(base)
				};

				IndirectAccess {
					base: reg_base,
					index: reg_index,
					scale: 1 << scale,
					offset: 0,
				}
			} else {
				if mode == 0 && rm & 7 == 5 { // RIP relative
					panic!("RIP relative");
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
				0 | 3 => 0,
				1 => read_imm(S8),
				2 => read_imm(S32),
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

	let opts = |opts: &[OpOption]| {
		let mut allowed = true;
		for opt in opts.iter() {
			let opsize = sr().operand_size;
			let l_allowed = match *opt {
				ImmSize(size) => {
					s().imm_size = size;
					true
				}
				ImmSizeOp => {
					s().imm_size = opsize;
					true
				}
				OpSize(size) => {
					s().operand_size = size;
					true
				}
				Term => {
					s().terminating = true;
					true
				}
				NoMem => {
					s().no_mem = true;
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
				Imm => {
					let imm_size = sr().imm_size;
					let imm = read_imm(imm_size);
					s().operands.push((Operand::Imm((imm, imm_size)), opsize));
					true
				}
				Branch => {
					s().branch = true;
					true
				}
				Disp => {
					let imm_size = sr().imm_size;
					let off = read_imm(imm_size) + s().cursor.offset as u64 as i64;
					s().branch = true;
					s().operands.push((Operand::Imm((off, imm_size)), opsize));
					true
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

	macro_rules! op {
		($code:expr, $name:expr, $opts:expr) => ({
			let state = sr().clone();
			if sr().cursor.data[sr().cursor.offset..].starts_with(&$code) {
				s().cursor.offset += $code.len();
				if opts(&$opts) {
					*in_cursor = sr().cursor.clone();
					let ops = sr().operands.iter().enumerate().map(|(i, _)| print_op(i)).collect::<Vec<String>>().join(", ");
					return Some(Instruction {
						desc: format!("{}{}{}", $name, if sr().operands.is_empty() { "" } else { " " }, ops),
						ops: sr().operands.clone(),
						branch: sr().branch,
						terminating: sr().terminating,
					});
				} else {
					*s() = state;
				}
			}
		})
	}

	macro_rules! pair {
		($code:expr, $name:expr, $opts:expr) => ({
			let mut o = Vec::new();
			o.push(OpSize(S8));
			o.extend($opts.iter().cloned());
			op!([$code], $name, *o);
			op!([$code + 1], $name, $opts);
		})
	}

	for (arith_opcode, instr) in ["add", "or", "adc", "sbb",
										"and", "sub", "xor", "cmp"].iter().enumerate() {
		for (format_num, format) in [[Rm, Reg],
											 [Reg, Rm],
											 [FixReg(0), Imm]].iter().enumerate() {
			let opcode = cat_bits(&[arith_opcode, format_num, 0], &[5, 2, 1]);
			pair!(opcode, instr, *format)
		}

		pair!(0x80, instr, [RmOpcode(arith_opcode), Imm]);
		op!([0x83], instr, [RmOpcode(arith_opcode), ImmSize(S8), Imm]);
	}

	for &(instr, opcode) in &[("rol", 0), ("ror", 1), ("rcl", 2), ("rcr", 3), ("shl", 4), ("shr", 5), ("sar", 7)] {
		pair!(0xc0, instr, [RmOpcode(opcode), ImmSize(S8), Imm]);
		pair!(0xd0, instr, [RmOpcode(opcode), FixImm(1)]);
		pair!(0xd2, instr, [RmOpcode(opcode), FixReg(1)]);
	}

	for (jmp_opcode, instr) in ["jc", "jnb", "jz", "jnz", "jbe", "jnbe", "js", "jns", "jp", "jnp", "jl", "jnl", "jle", "jnle"].iter().enumerate() {
		op!([0x72 + jmp_opcode as u8], instr, [ImmSize(S8), Disp]);
		op!([0x0F, 0x82 + jmp_opcode as u8], instr, [OpSize(S32), Disp]); // 16/32 opsize
	}

	pair!(0xf6, "test", [RmOpcode(0), Imm]);
	for &(instr, opcode) in &[("not", 2), ("neg", 3), ("mul", 4), ("imul", 5), ("div", 6), ("idiv", 7)] {
		pair!(0xf6, instr, [RmOpcode(opcode), Imm]);
	}

	op!([0x0f, 0x1f], "nop", [RmOpcode(0), NoMem]);

	op!([0xeb], "jmp", [OpSize(S8), Term, Disp]);
	op!([0xe9], "jmp", [OpSize(S32), Term, Disp]); // 16/32 opsize
	op!([0xff], "jmp", [OpSize(S64), RmOpcode(4), Term, Branch]);

	op!([0xe8], "call", [OpSize(S32), Disp]); // 16/32 opsize
	op!([0xff], "call", [OpSize(S64), RmOpcode(2), Branch]);

	for reg in 0..8 {
		op!([0x50 + reg], "push", [OpSize(S64), FixRegRex(reg as usize)]);
		op!([0x58 + reg], "pop", [OpSize(S64), FixRegRex(reg as usize)]);
	}

	pair!(0x88, "mov", [Rm, Reg]);
	pair!(0x8a, "mov", [Reg, Rm]);
	pair!(0xc6, "mov", [RmOpcode(0), Imm]);
	pair!(0xa0, "mov", [FixReg(0), Imm]);
	pair!(0xa2, "mov", [Imm, FixReg(0)]);

	for reg in 0..8 {
		op!([0xb0 + reg], "mov", [OpSize(S8), FixReg(reg as usize), ImmSize(S8), Imm]);
		op!([0xb8 + reg], "mov", [FixRegRex(reg as usize), ImmSizeOp, Imm]);
	}

	op!([0x8d], "lea", [Reg, Rm, NoMem]);

	op!([0xc3], "ret", [Term]);

	op!([0x0f, 0xb6], "movzx", [Reg, OpSize(S8), Rm]);

	None
}