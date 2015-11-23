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

struct IndirectAccess {
	base_reg: usize,
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
	Indirect(String, i64),
	Imm(i64),
}

#[derive(Clone)]
enum OpOption {
	Rm,
	FixReg(usize),
	FixRegRex(usize),
	Imm,
	Reg,
	Disp,
	Branch,
	RmOpcode(usize),
	OpSize(Size),
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
	operand_size: Size,
	operands: Vec<(Operand, Size)>,
	imm_size: Size,
	terminating: bool,
	modrm_cache: Option<(Operand, usize)>,
	branch: bool,
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

pub fn parse(in_cursor: &mut Cursor, rex: Option<u8>) -> Option<Instruction> {
	let rex = rex.unwrap_or(0);
	let rex_w = ext_bit(rex as usize, 3, 0) != 0;
	let rex_b = ext_bit(rex as usize, 0, 0) != 0;

	let state = RefCell::new(State {
		cursor: in_cursor.clone(),
		terminating: false,
		operands: Vec::new(),
		operand_size: if rex_w { S64 } else { S32 },
		imm_size: S32,
		modrm_cache: None,
		branch: false,
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
			Operand::Indirect(name, offset) => {
				if offset != 0 {
					format!("{}[{}{}]", operand_ptr(known_size, op.1), name, sign_hex(offset, true))
				} else {
					format!("{}[{}]", operand_ptr(known_size, op.1), name)
				}
			}
			Operand::Imm(im) => sign_hex(im, false),
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

			let name = if rm & 7 == 4 { // SIB BYTE
				"sib"
			} else {
				if mode == 0 && rm & 7 == 5 { // RIP relative
					"rip"
				} else {
					REGS64[rm]
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
				Operand::Indirect(name.to_string(), off)
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
				OpSize(size) => {
					s().operand_size = size;
					true
				}
				Term => {
					s().terminating = true;
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
				Imm => {
					let imm_size = sr().imm_size;
					let imm = read_imm(imm_size);
					s().operands.push((Operand::Imm(imm), opsize));
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
					s().operands.push((Operand::Imm(off), opsize));
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
			o.push(ImmSize(S8));
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

	for (jmp_opcode, instr) in ["jc", "jnb", "jz", "jnz", "jbe", "jnbe", "js", "jns", "jp", "jnp", "jl", "jnl", "jle", "jnle"].iter().enumerate() {
		op!([0x72 + jmp_opcode as u8], instr, [ImmSize(S8), Disp]);
		op!([0x0F, 0x82 + jmp_opcode as u8], instr, [OpSize(S32), Disp]); // 16/32 opsize
	}

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

	op!([0x8d], "lea", [Reg, Rm]);

	op!([0xc3], "ret", [Term]);


	op!([0x0f, 0xb6], "movzx", [Reg, OpSize(S8), Rm]);
/*
	for reg in 0..8 {
		op!(0xb0 + reg, "mov", rm);
	}
*/
	None
}