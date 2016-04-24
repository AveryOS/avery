use decoder::Cursor;
use std::cell::RefCell;
use std::cmp;
use std::iter;
use effect::{DecodedOperand, Regs, Inst, IndirectAccess, RT, Mem, Disp, Effect, Operand, Size};
use effect::Size::*;
use table::{self, bytes, P_LOCK, P_SEG_GS, P_SEG_FS, P_OP_SIZE, P_REP};

pub static mut DEBUG: bool = false;

macro_rules! debug {
    ($($arg:tt)*) => (
        if unsafe { DEBUG } {
            print!($($arg)*);
        }
    );
}

fn ext_bit(b: usize, i: usize, t: usize) -> usize {
	((b >> i) & 1) << t
}

fn sign_hex(i: i64, plus: bool) -> String {
	if i < 0 {
		format!(" - {:#x}", -i)
	} else {
		format!("{}{:#x}", if plus { " + " } else { "" }, i)
	}
}

#[derive(Clone)]
struct State<'s> {
	cursor: Cursor<'s>,
	inst: Inst,
	modrm_cache: Option<(DecodedOperand, usize)>,
}

const REGS_CR: &'static [&'static str] = &["cr0", "cr1", "cr2", "cr3", "cr4", "cr5", "cr6", "cr7",
	"cr8", "cr9", "cr10", "cr11", "cr12", "cr13", "cr14", "cr15"]; 
const REGS_MMX: &'static [&'static str] = &["mm0", "mm1", "mm2", "mm3", "mm4", "mm5", "mm6", "mm7",
	"mm8", "mm9", "mm10", "mm11", "mm12", "mm13", "mm14", "mm15"]; 
const REGS_SSE: &'static [&'static str] = &["xmm0", "xmm1", "xmm2", "xmm3", "xmm4", "xmm5", "xmm6", "xmm7",
	"xmm8", "xmm9", "xmm10", "xmm11", "xmm12", "xmm13", "xmm14", "xmm15"]; 
const REGS64: &'static [&'static str] = &["rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi",
	  "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15",
	  "rip"];  // RIP appended
const REGS32: &'static [&'static str] = &["eax", "ecx", "edx", "ebx", "esp", "ebp", "esi", "edi",
		  "r8d", "r9d", "r10d", "r11d", "r12d", "r13d", "r14d", "r15d"];
const REGS16: &'static [&'static str] = &["ax", "cx", "dx", "bx", "sp", "bp", "si", "di",
		  "r8w", "r9w", "r10w", "r11w", "r12w", "r13w", "r14w", "r15w"];
const REGS8 : &'static [&'static str] = &["al", "cl", "dl", "bl", "spl", "bpl", "sil", "dil",
				  "r8b", "r9b", "r10b", "r11b", "r12b", "r13b", "r14b", "r15b"];
const REGS8_NOREX : &'static [&'static str] = &["al", "cl", "dl", "bl", "ah", "ch", "dh", "bh"];

fn operand_ptr(size_known: bool, op_size: Size) -> &'static str {
	if size_known {
		return "";
	};
	match op_size {
		S128 => "xmmword ptr ",
		S64 => "qword ptr ",
		S32 => "dword ptr ",
		S16 => "word ptr ",
		S8 => "byte ptr ",
		_ => panic!(),
	}
}

fn reg_name(r: RT, op_size: Size, rex: bool) -> &'static str  {
	match r {
		RT::GP(r) => match op_size {
			S128 => panic!(),
			S64 => REGS64[r],
			S32 => REGS32[r],
			S16 => REGS16[r],
			S8 => if rex { REGS8[r] } else { REGS8_NOREX[r] },
			_ => panic!(),
		},
		RT::CR(r) => REGS_CR[r],
		RT::SSE(r) => REGS_SSE[r],
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
		_ => panic!(),
	}
}

pub fn gen_all(inst: &Inst, cases: &mut Vec<(Vec<u8>, Vec<Effect>)>) {
	let name = &inst.name[..];

	for prefixes in &[
		&[][..], &[P_LOCK], &[P_LOCK, P_SEG_GS], &[P_SEG_GS],
		&[P_OP_SIZE], &[P_OP_SIZE, P_LOCK], &[P_OP_SIZE, P_LOCK, P_SEG_GS], &[P_OP_SIZE, P_SEG_GS]
	] {
		for rex_byte in (0x41..0x48u8).map(|r| Some(r)).chain(iter::once(None)) {
			if inst.prefix_bytes.iter().any(|p| prefixes.contains(p)) {
				continue;
			}

			if prefixes.iter().any(|p| !inst.prefix_whitelist.contains(p)) {
				continue;
			}

			if rex_byte.is_some() {
				continue;
			}

			let rex = rex_byte.unwrap_or(0);
			let rex_w = ext_bit(rex as usize, 3, 0) != 0;
			let rex_b = ext_bit(rex as usize, 0, 0) != 0;
			let operand_size_override = prefixes.contains(&P_OP_SIZE);

			let op_size = if rex_w { S64 } else { if operand_size_override { S16 } else { S32 } };
			let imm_size = if op_size == S16 { S16 } else { S32 };

			let decode_size = |s: Size| {
				match s {
					SMMXSize => if operand_size_override { S128 } else { S64 },
					SRexSize => if rex_w { S64 } else { S32 },
					SImmSize => imm_size,
					SOpSize => op_size,
					_ => s
				}
			};

			let mut modrm_type = None;
			let mut mod_rm_ro = inst.read_only;

			for (i, op) in inst.operands.iter().enumerate() {
				let ro = i >= 1;

				match *op {
					(Operand::Rm(_), _) => {
						if ro {
							mod_rm_ro = true;
						}
						modrm_type = Some(None);
					}
					(Operand::Reg(_), _) => {
						modrm_type = Some(None);
					}
					(Operand::RmOpcode(reg), _) => {
						modrm_type = Some(Some(Some(reg)));
					}
					(Operand::Mem(_), _) => {
						if ro {
							mod_rm_ro = true;
						}
						modrm_type = Some(Some(None));
					}
					_ => {}
				}
			}

			let mut modrms = vec![];

			if let Some(kind) = modrm_type {
				for modrm in 0..256usize {
					let mode = modrm >> 6;
					let reg = ((modrm >> 3) & 7) | ext_bit(rex as usize, 2, 3);
					let rm = modrm & 7 | ext_bit(rex as usize, 0, 3);

					if let Some(Some(opcode)) = kind {
						if opcode != reg {
							continue;
						}
					}

					if mode == 3 && Some(None) == kind {
						continue;
					}

					let indir = if mode != 3 && rm & 7 == 4 {
						// SIB byte

						continue;
					} else {
						if mode == 0 && rm & 7 == 5 { // RIP relative
							Some(Mem::Rip)
						} else if mode == 3 { // reg
							None
						} else {
							Some(match mode {
								0 => Mem::Mem(rm, Disp::None),
								1 => Mem::Mem(rm, Disp::Imm8),
								2 => Mem::Mem(rm, Disp::Imm32),
								_ => panic!(),
							})
						}
					};

					modrms.push(Some((modrm, indir.ok_or(rm), reg)));
				}
			} else {
				modrms.push(None);
			}

			'inner: for modrm in modrms {
				if prefixes.contains(&P_LOCK) {
					match modrm {
						Some((_, Ok(_), _)) if !mod_rm_ro => (),
						_ => continue,
					}
				}

				let mut effects = Vec::new();

				let mut i = 0;
				while i < inst.operands.len() {
					let ops = &inst.operands[i..];
					let ro = i >= 1 || inst.read_only;

					macro_rules! get {
					    ($mem:expr, $reg:expr) => (match modrm {
							Some((_, Ok(mem), other)) => $mem(mem, other),
							Some((_, Err(reg), other)) => $reg(reg, other),
							None => panic!(),
						})
					}

					let get_write = || get!(|m, _| Effect::WriteMem(m), |r, _| Effect::ClobReg(r));
					let get_read = || get!(|m, _| Effect::CheckMem(m), |_, _| Effect::None);

					i += match ops {
						[(Operand::FixRegRex(mut reg, Regs::GP), _), ..] if name == "push" => {
							if rex_b {
								reg += 8;
							}
							effects.push(Effect::Push(reg));
							1
						} 
						[(Operand::FixRegRex(mut reg, Regs::GP), _), ..] if name == "pop" => {
							if rex_b {
								reg += 8;
							}
							effects.push(Effect::Pop(reg));
							1
						} 
						[(Operand::FixRegRex(mut reg, Regs::GP), _), ..] if !ro => {
							if rex_b {
								reg += 8;
							}
							effects.push(Effect::ClobReg(reg));
							1
						}
						[(Operand::FixReg(_, _), _), ..] if ro => 1,
						[(Operand::FixReg(c, Regs::GP), _), ..] if !ro => {
							effects.push(Effect::ClobReg(c));
							1
						}

						[(Operand::Clob(c), _), ..] =>  {
							effects.push(Effect::ClobReg(c));
							1
						}

						[(Operand::FixImm(_, _), _), ..] => 1,

						[(Operand::Disp(S8), _), ..] => {
							effects.push(Effect::Jmp8);
							1
						}
						[(Operand::Disp(S32), _), ..] =>  {
							effects.push(if name == "call" { Effect::Call32 } else { Effect::Jmp32 });
							1
						}
						
						[(Operand::Addr, _), ..] => {
							effects.push(Effect::CheckAddr);
							1
						}
						
						[(Operand::Imm(size), _), ..] => {
							effects.push(match decode_size(size) {
								S64 => Effect::Imm64,
								S32 => Effect::Imm32,
								S16 => Effect::Imm16,
								S8 => Effect::Imm8,
								_ => panic!(),
							});
							1
						}

						[(Operand::RmOpcode(_), _), ..] => {
							effects.push(if name == "call" {
								get!(|m, _| Effect::Call(m), |_, _| Effect::None) // TODO: CFI
							} else {
								get_write()
							});
							1
						}

						[(Operand::Rm(Regs::GP), _), (Operand::Reg(regs), _), ..] => {
							effects.push(if inst.read_only {
								get_read()
							} else if name == "mov" && regs == Regs::GP { 
								get!(|m, o| Effect::Store(m, o), |r, o| Effect::Move(r, o))
							} else {
								get_write()
							});
							2
						}
						[(Operand::Reg(Regs::GP), _), (Operand::Rm(regs), _), ..] => {
							effects.push(if inst.read_only {
								get_read()
							} else if name == "mov" && regs == Regs::GP { 
								get!(|m, o| Effect::Load(o, m), |r, o| Effect::Move(o, r))
							} else {
								get_write()
							});
							2
						}
						[(Operand::Rm(_), _), (Operand::Reg(_), _), ..] => {
							effects.push(get_read());
							2
						}
						[(Operand::Reg(_), _), (Operand::Rm(_), _), ..] => {
							effects.push(get_read());
							2
						} 
						[(Operand::Reg(Regs::GP), _), (Operand::Mem(None), _), ..] => {
							assert!(name == "lea");
							effects.push(get!(|m, _| Effect::Lea(m), |_, _| panic!()));
							2
						}
						_ => {
							println!("Unknown ops {:?} on {:?}!", ops, inst);
							continue 'inner;
						},
					}
				}

				let mut bytes = Vec::new();

				bytes.extend_from_slice(prefixes);
				rex_byte.map(|r| bytes.push(r));
				bytes.extend_from_slice(&inst.prefix_bytes);
				bytes.extend_from_slice(&inst.bytes);
				if let Some((b, _, _)) = modrm {
					bytes.push(b as u8);
				}


				debug!("Adding {} ({}) => {:?}\n", name, table::bytes(&bytes), effects);

				cases.push((bytes, effects));
			}
		}
	}
}

pub fn parse(in_cursor: &mut Cursor, rex: Option<u8>, prefixes: &[u8], disp_off: u64, insts: &[Inst]) -> Option<Inst> {	let rex = rex.unwrap_or(0);
	let rex_w = ext_bit(rex as usize, 3, 0) != 0;
	let rex_b = ext_bit(rex as usize, 0, 0) != 0;
	let operand_size_override = prefixes.contains(&P_OP_SIZE);
	let fs_override = prefixes.contains(&P_SEG_FS);
	let gs_override = prefixes.contains(&P_SEG_GS);

	if gs_override && fs_override {
		return None;
	}

	let op_size = if rex_w { S64 } else { if operand_size_override { S16 } else { S32 } };
	let imm_size = if op_size == S16 { S16 } else { S32 };

	let input = &in_cursor.remaining();
	let input = &input[..cmp::min(16, input.len())];

	debug!("Decoding instruction {} rex_w: {}, op_override: {}, opsize: {:?}, prefixes: {}\n", bytes(input), rex_w, operand_size_override, op_size, bytes(prefixes));

	let i = insts.iter().find(|i| {
		if in_cursor.data[in_cursor.offset..].starts_with(&i.bytes) {
			debug!("t {} instruction {} rex_w: {}, op_override: {}, opsize: {:?}, prefixes: {}\n", i.name, bytes(input), rex_w, operand_size_override, op_size, bytes(prefixes));
		}
		if in_cursor.data[in_cursor.offset..].starts_with(&i.bytes) && prefixes.ends_with(&i.prefix_bytes[..]) {
			if let Some(o) = i.opcode {
				((in_cursor.remaining()[i.bytes.len()] >> 3) & 7) as usize == o
			} else {
			    true
			}
		} else {
			false
		}
	});
	let i = if let Some(i) = i { i.clone() } else { return None };

	debug!("Testing instruction {:?}\n", i);

	let len = i.bytes.len();

	let state = RefCell::new(State {
		cursor: in_cursor.clone(),
		inst: i,
		modrm_cache: None,
	});

	let sr = || state.borrow();
	let s = || state.borrow_mut();

	s().cursor.offset += len;

	let decode_size = |s: Size| {
		match s {
			SMMXSize => if operand_size_override { S128 } else { S64 },
			SRexSize => if rex_w { S64 } else { S32 },
			SImmSize => imm_size,
			SOpSize => op_size,
			_ => s
		}
	};

	let has_prefix = |p: u8| {
		if sr().inst.prefix_bytes.contains(&p) {
			false
		} else {
			prefixes.contains(&p)
		}
	};

	let print_op = |i: usize| {
		let op = sr().inst.decoded_operands[i].clone();

		match op.0 {
			DecodedOperand::Direct(reg) => format!("{}", reg_name(reg, op.1, rex != 0)),
			DecodedOperand::Indirect(indir) => {
				let ptr = operand_ptr(sr().inst.no_mem, op.1);

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
					&(Some(base), Some(index)) => format!("{} + {}{}", REGS64[base], REGS64[index], scale),
					&(None, Some(index)) => format!("{}{}", REGS64[index], scale),
					&(Some(base), None) => format!("{}", REGS64[base]),
					&(None, None) => {
						return if indir.offset_wide {
							format!("{}{}[{:#x}]", ptr, segment, indir.offset)
						} else {
							format!("{}{}[{:#x}]", ptr, segment, indir.offset as i32)
						}
					},
				};

				if indir.offset != 0 {
					format!("{}{}[{}{}]", ptr, segment, name, sign_hex(indir.offset, true))
				} else {
					format!("{}{}[{}]", ptr, segment, name)
				}
			}
			DecodedOperand::Imm(im, _) => match op.1 {
				Lit1 => format!("{}", im as i8),
				S8 => format!("{:#x}", im as i8),
				S16 => format!("{:#x}", im as i16),
				S32 => format!("{:#x}", im as i32),
				S64 => format!("{:#x}", im),
				_ => panic!(),
			}
		}
	};

	let reg_ref = |r: usize, regs: Regs| {
		match regs {
			Regs::GP => RT::GP(r),
			Regs::SSE => RT::SSE(r),
			_ => panic!(),
		}
	};

	let modrm = |regs: Regs| {
		if !sr().modrm_cache.is_some() {
			let modrm = s().cursor.next() as usize;
			let mode = modrm >> 6;
			let reg = ((modrm >> 3) & 7) | ext_bit(rex as usize, 2, 3);
			let rm = modrm & 7 | ext_bit(rex as usize, 0, 3);

			//println!("mode:{} reg:{} rm: {}", mode ,reg ,rm);

			let mut name = if mode != 3 && rm & 7 == 4 {
				println!("\nSIB-byte used!\n");
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
					offset_wide: false,
				}
			} else {
				if mode == 0 && rm & 7 == 5 { // RIP relative
					let off = read_imm(&state, S32);

					IndirectAccess {
						base: Some(16), // RIP
						index: None,
						scale: 0,
						offset: off,
						offset_wide: false,
					}
				} else {
					IndirectAccess {
						base: Some(rm),
						index: None,
						scale: 0,
						offset: 0,
						offset_wide: false,
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
				DecodedOperand::Direct(RT::GP(rm))
			} else {
				name.offset = off;
				DecodedOperand::Indirect(name)
			};

			s().modrm_cache = Some((indir, reg));
		}

		match sr().modrm_cache.as_ref().unwrap().clone() {
			(DecodedOperand::Direct(RT::GP(v)), s) => (DecodedOperand::Direct(reg_ref(v, regs)), s),
			v => v,
		}
	};

	let len = sr().inst.operands.len();

	for i in 0..len {
		let op = sr().inst.operands[i].clone();
		match op {
			(Operand::Imm(size), op_size) => {
				let imm = read_imm(&state, decode_size(size));
				s().inst.decoded_operands.push((DecodedOperand::Imm(imm, decode_size(size)), decode_size(op_size)));
			}
			(Operand::FixImm(imm, size), op_size) => {
				s().inst.decoded_operands.push((DecodedOperand::Imm(imm, decode_size(size)), decode_size(op_size)));
			}
			(Operand::Disp(size), _) => {
				let off = read_imm(&state, decode_size(size)).wrapping_add(disp_off as i64).wrapping_add(sr().cursor.offset as u64 as i64);
				s().inst.decoded_operands.push((DecodedOperand::Imm(off, decode_size(size)), S64));
			}
			(Operand::FixReg(reg, regs), op_size) => {
				let r = reg_ref(reg, regs);
				s().inst.decoded_operands.push((DecodedOperand::Direct(r), decode_size(op_size)));
			}
			(Operand::FixRegRex(mut reg, regs), op_size) => {
				if rex_b {
					reg += 8;
				}
				let r = reg_ref(reg, regs);
				s().inst.decoded_operands.push((DecodedOperand::Direct(r), decode_size(op_size)));
			}
			(Operand::Clob(_), _) => {}
			(Operand::Addr, op_size) => {
				let a = IndirectAccess {
					base: None,
					index: None,
					scale: 0,
					offset: read_imm(&state, S64),
					offset_wide: true,
				};
				s().inst.decoded_operands.push((DecodedOperand::Indirect(a), decode_size(op_size)));
			}
			(Operand::Rm(regs), op_size) => {
				let (indir, _) = modrm(regs);
				s().inst.decoded_operands.push((indir, decode_size(op_size)));
			}
			(Operand::Reg(regs), op_size) => {
				let (_, reg) = modrm(regs);
				let r = reg_ref(reg, regs);
				s().inst.decoded_operands.push((DecodedOperand::Direct(r), decode_size(op_size)));
			}
			(Operand::RmOpcode(_), op_size) => {
				let (indir, _) = modrm(Regs::GP);
				s().inst.decoded_operands.push((indir, decode_size(op_size)));
			}
			(Operand::Mem(_), op_size) => {
				let (indir, _) = modrm(Regs::GP);
				match indir {
					DecodedOperand::Indirect(..) => {
						s().inst.decoded_operands.push((indir, decode_size(op_size)));
					}
					_ => panic!(),
				}
			}
		}
	}

	let valid_state = || {
		let mut prefix = String::new();

		if has_prefix(P_LOCK) {
			prefix.push_str("lock ");
		}

		if has_prefix(P_REP) {
			prefix.push_str("rep ");
		}

		if !sr().inst.no_mem {
			let len = sr().inst.decoded_operands.len();
			for i in 0..len {
				let op = sr().inst.decoded_operands[i].0.clone();
				match op {
					DecodedOperand::Direct(..) | DecodedOperand::Indirect(..) => s().inst.prefix_whitelist.push(P_OP_SIZE),
					_ => ()
				}
				match op {
					DecodedOperand::Indirect(..) => {
						s().inst.prefix_whitelist.push(P_SEG_GS);
						s().inst.prefix_whitelist.push(P_SEG_FS);
					}
					_ => ()
				}
			}
		}

		if prefixes.iter().all(|p| {
			let r = sr().inst.prefix_whitelist.contains(p) || sr().inst.prefix_bytes.contains(p);
			if !r {
				print!("Prefix {:02x} not allowed on instruction\n", p);
			}
			r
		}) {
			Some(prefix)
		} else {
			None
		}
	};

	let mut i = sr().inst.clone();

	if i.name == "cwde" && rex_w {
		i.name = "cdqe".to_string();
	} else if i.name == "cdq" && rex_w {
		i.name = "cqo".to_string();
	}

	*in_cursor = sr().cursor.clone();

	let op_size_postfix = sr().inst.op_size_postfix;
	let iname = if op_size_postfix {
		s().inst.prefix_whitelist.push(P_OP_SIZE);
		i.name.clone() + match decode_size(sr().inst.operand_size) {
			S128 => panic!(),
			S64 => "q",
			S32 => "d",
			S16 => "w",
			S8 => "b",
			_ => panic!(),
		}
	} else {
		i.name.clone()
	};
	let prefix = if let Some(p) = valid_state() { p } else { return None };
	let ops = sr().inst.decoded_operands.iter().enumerate().map(|(i, _)| print_op(i)).collect::<Vec<String>>().join(", ");
	i.desc = format!("{}{}{}{}", prefix, iname, if sr().inst.decoded_operands.is_empty() { "" } else { " " }, ops);
	Some(i)
}
