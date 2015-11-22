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
}

struct IndirectAccess {
	base_reg: usize,
	offset: i64,
}

#[derive(Clone)]
enum ModRM {
	Direct(usize),
	Indirect(String, i64),
}

enum OpOption {
	Rm,
	Ax,
	Imm,
	Reg,
	Branch,
	RmOpcode(usize),
	OpSize(usize),
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

pub fn parse(cursor: &mut Cursor, rex: Option<u8>) -> Option<Instruction> {
	let rex = rex.unwrap_or(0);
	let operands: RefCell<Vec<String>> = RefCell::new(Vec::new());
	let regs64 = &["rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi",
		  "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15"];
	let regs32 = ["eax", "ecx", "edx", "ebx", "esp", "ebp", "esi", "edi",
			  "r8d", "r9d", "r10d", "r11d", "r12d", "r13d", "r14d", "r15d"];
	let regs16 = ["ax", "cx", "dx", "bx", "sp", "bp", "si", "di",
			  "r8w", "r9w", "r10w", "r11w", "r12w", "r13w", "r14w", "r15w"];
	let regs8 = ["al", "cl", "dl", "bl", "spl", "bpl", "sil", "dil",
					  "r8b", "r9b", "r10b", "r11b", "r12b", "r13b", "r14b", "r15b"];
	let rex_w = ext_bit(rex as usize, 3, 0) != 0;
	let i = RefCell::new(Instruction {
		desc: "".to_string(),
		terminating: false,
	});
	let operand_size = RefCell::new(if rex_w { 64 } else { 32 });
	let cursor = RefCell::new(cursor);

	let cr = || cursor.borrow();
	let c = || cursor.borrow_mut();

	let op_ptr = || {
		match *operand_size.borrow() {
			64 => "",
			32 => "dword ",
			16 => "word ",
			8 => "byte ",
			_ => panic!(),
		}
	};

	let reg_name = |r: usize| {
		match *operand_size.borrow() {
			64 => regs64[r],
			32 => regs32[r],
			16 => regs16[r],
			8 => regs8[r],
			_ => panic!(),
		}
	};

	let modrm_s = RefCell::new(None);

	let print_modrm = |rm: ModRM| {
		match rm {
			ModRM::Direct(reg) => format!("{}", reg_name(reg)),
			ModRM::Indirect(name, offset) => {
				if offset != 0 {
					format!("{}[{}+{:#x}]", op_ptr(), name, offset)
				} else {
					format!("{}[{}]", op_ptr(), name)
				}
			}
		}
	};

	let modrm = || {
		let mut modrm_s = modrm_s.borrow_mut();
		if !modrm_s.is_some() {
			let modrm = c().next() as usize;

			let mode = modrm >> 6;
			let reg = ((modrm >> 3) & 7) | ext_bit(rex as usize, 2, 3);
			let rm = modrm & 7 | ext_bit(rex as usize, 0, 3);

			let name = if rm & 7 == 4 { // SIB BYTE
				"sib"
			} else {
				if rm & 7 == 5 { // RIP relative
					"rip"
				} else {
					regs64[rm]
				}
			};

			let off = match mode {
				0 | 3 => 0,
				1 => {
					c().next() as i8 as i64
				}
				2 => {
					let mut v = c().next() as u32;
					v |= (c().next() as u32) << 8;
					v |= (c().next() as u32) << 8;
					v |= (c().next() as u32) << 8;
					v as i32 as i64
				}
				_ => panic!(),
			};

			let indir = if mode == 3 {
				ModRM::Direct(rm)
			} else {
				ModRM::Indirect(name.to_string(), off)
			};

			*modrm_s = Some((indir, reg));
		}
		modrm_s.as_ref().unwrap().clone()
	};

	let read_imm = || {
		match *operand_size.borrow() {
			8 => {
				c().next() as i8 as i64
			}
			16 => {
				let mut v = c().next() as u32;
				v |= (c().next() as u32) << 8;
				v as i32 as i64
			}
			32 => {
				let mut v = c().next() as u32;
				v |= (c().next() as u32) << 8;
				v |= (c().next() as u32) << 8;
				v |= (c().next() as u32) << 8;
				v as i32 as i64
			}
			64 => {
				let mut v = c().next() as u64;
				v |= (c().next() as u64) << 8;
				v |= (c().next() as u64) << 8;
				v |= (c().next() as u64) << 8;
				v |= (c().next() as u64) << 8;
				v |= (c().next() as u64) << 8;
				v |= (c().next() as u64) << 8;
				v |= (c().next() as u64) << 8;
				v as i64
			}
			_ => panic!(),
		}
	};

	let opts = |opts: &[OpOption]| {
		let mut allowed = true;
		for opt in opts.iter() {
			let l_allowed = match *opt {
				OpSize(s) => {
					*operand_size.borrow_mut() = s;
					true
				}
				Term => {
					i.borrow_mut().terminating = true;
					true
				}
				Ax => {
					operands.borrow_mut().push(reg_name(0).to_string());
					true
				}
				Imm => {
					operands.borrow_mut().push(format!("{:#x}", read_imm()));
					true
				}
				Branch => {
					operands.borrow_mut().push(format!("{:#x}", read_imm() + c().offset as u64 as i64));
					true
				}
				Rm => {
					let (indir, _) = modrm();
					operands.borrow_mut().push(print_modrm(indir));
					true
				}
				Reg => {
					let (_, reg) = modrm();
					operands.borrow_mut().push(format!("{}", reg_name(reg)));
					true
				}
				RmOpcode(opcode_ext) => {
					let o = c().offset;
					let (indir, reg) = modrm();
					if reg & 7 != opcode_ext {
						c().offset = o;
						*modrm_s.borrow_mut() = None;
						false
					} else {
						operands.borrow_mut().push(print_modrm(indir));
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
			if cr().data[cr().offset..].starts_with(&$code) {
				c().offset += $code.len();
				if opts(&$opts) {
					i.borrow_mut().desc = format!("{}{}{}", $name, if operands.borrow().is_empty() { "" } else { " " }, operands.borrow().join(", "));
					return Some(i.borrow().clone());
				}
			}
		})
	}

	macro_rules! pair {
		($code:expr, $name:expr, $opts:expr) => ({
			op!([$code], $name, $opts);
			op!([$code + 1], $name, $opts);
		})
	}

	for (arith_opcode, instr) in ["add", "or", "adc", "sbb",
										"and", "sub", "xor", "cmp"].iter().enumerate() {
		for (format_num, format) in [[Rm, Reg],
											 [Reg, Rm],
											 [Ax, Imm]].iter().enumerate() {
			let opcode = cat_bits(&[arith_opcode, format_num, 0], &[5, 2, 1]);
			pair!(opcode, instr, *format)
		}

		pair!(0x80, instr, [RmOpcode(arith_opcode), Imm]);
		op!([0x83], instr, [OpSize(8), Rm, Imm]);
	}

	for (jmp_opcode, instr) in ["jc", "jnb", "jz", "jnz", "jbe", "jnbe", "js", "jns", "jp", "jnp", "jl", "jnl", "jle", "jnle"].iter().enumerate() {
		op!([0x72 + jmp_opcode as u8], instr, [OpSize(8), Branch]);
		op!([0x0F, 0x82 + jmp_opcode as u8], instr, [OpSize(32), Branch]); // 16/32 opsize
	}

	op!([0xeb], "jmp", [OpSize(8), Branch]);
	op!([0xe9], "jmp", [OpSize(32), Branch]); // 16/32 opsize
	op!([0xff], "jmp", [OpSize(64), Term, RmOpcode(4)]);

	pair!(0x88, "mov", [Rm, Reg]);
	pair!(0x8a, "mov", [Reg, Rm]);

	pair!(0xc3, "ret", [Term]);


	op!([0x0f, 0xb6], "movzx", [Reg, OpSize(8), Rm]);
/*
	for reg in 0..8 {
		op!(0xb0 + reg, "mov", rm);
	}
*/
	None
}