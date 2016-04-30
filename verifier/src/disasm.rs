use decoder::Cursor;
use std::cell::RefCell;
use std::iter;
use effect::*;
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
	operands: Vec<(DecodedOperand, Size)>,
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

fn read_imm(c: &mut Cursor, size: Size) -> i64 {
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

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum ModRM {
	Reg(usize),
	Indirect(IndirectAccessFormat),
	Sib(Disp, usize),
}

pub fn gen_all(inst: &Inst, cases: &mut Vec<(Vec<u8>, Vec<Effect>, InstFormat)>) {
	let name = &inst.name[..];
	//unsafe { DEBUG = name == "xadd"; };
	debug!("Generating all {} {}\n", name, table::bytes(&inst.bytes));

	for prefixes in &[
		&[][..], &[P_LOCK], &[P_SEG_GS, P_LOCK], &[P_SEG_GS],
		&[P_OP_SIZE], &[P_OP_SIZE, P_LOCK], &[P_SEG_GS, P_OP_SIZE, P_LOCK], &[P_SEG_GS, P_OP_SIZE]
	] {
		if inst.prefix_bytes.iter().any(|p| prefixes.contains(p)) {
			debug!("Rejected prefixes {:?}: Contained in opcode\n", table::bytes(prefixes));
			continue;
		}

		if prefixes.contains(&P_LOCK) && !inst.prefix_whitelist.contains(&P_LOCK) {
			debug!("Rejected prefixes {:?}: P_LOCK not allowed\n", table::bytes(prefixes));
			continue;
		}

		for rex_byte in (0x41...0x4Fu8).map(|r| Some(r)).chain(iter::once(None)) {
			// REX doesn't work if the instruction uses prefixes as the opcode
			if rex_byte.is_some() && !inst.prefix_bytes.is_empty() {
				continue;
			}

			let rex_val = rex_byte.unwrap_or(0) as usize;
			
			let mut modrm_type = None;
			let mut mod_rm_ro = false;

			for op in &inst.operands {
				match *op {
					(Operand::Rm(_), _, access) => {
						if access == Access::Read {
							mod_rm_ro = true;
						}
						modrm_type = Some(None);
					}
					(Operand::Reg(_), _, _) => {
						modrm_type = Some(None);
					}
					(Operand::RmOpcode(reg), _, _) => {
						modrm_type = Some(Some(Some(reg)));
					}
					(Operand::Mem(_), _, access) => {
						if access == Access::Read {
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
					let reg_rex = ext_bit(rex_val, 2, 3);
					let reg = ((modrm >> 3) & 7) | reg_rex;
					let rm_rex = ext_bit(rex_val, 0, 3);
					let rm = modrm & 7 | rm_rex;
					let mut use_rex = (reg_rex | rm_rex) != 0;

					if let Some(Some(opcode)) = kind {
						if opcode != reg {
							continue;
						}
					}

					if mode == 3 && Some(None) == kind {
						continue;
					}

					let disp = match mode {
						0 => Some(Disp::None),
						1 => Some(Disp::Imm8),
						2 => Some(Disp::Imm32),
						_ => None,
					};

					let indir = if mode != 3 && rm & 7 == 4 {
						ModRM::Sib(disp.unwrap(), mode)
					} else {
						if mode == 0 && rm & 7 == 5 { // RIP relative
							use_rex = reg_rex != 0; // rm rex bit not used here
							ModRM::Indirect(IndirectAccessFormat {
								base: Some(16),
								index: None,
								scale: 0,
								disp: Disp::Imm32,
							})
						} else if mode == 3 { // reg
							ModRM::Reg(rm)
						} else {
							ModRM::Indirect(IndirectAccessFormat {
								base: Some(rm),
								index: None,
								scale: 0,
								disp: disp.unwrap(),
							})
						}
					};

					// TODO: Ensure that the values dervied from the REX bytes are actually used in the instruction

					modrms.push(Some((modrm, indir, reg, use_rex)));
				}
			} else {
				modrms.push(None);
			}

			'inner: for modrm in modrms {
				if prefixes.contains(&P_LOCK) {
					match modrm {
						Some((_, ModRM::Indirect(..), _, _)) if !mod_rm_ro => (),
						_ => continue,
					}
				}

				let sibs = match modrm {
					Some((_, ModRM::Sib(..), _, _)) => 256usize,
					_ => 1,
				};

				for sib in 0..sibs {
					let sib = if sibs > 1 { Some(sib) } else { None };

					let rex_x_set = ext_bit(rex_val, 1, 0) != 0;

					// Are we doing anything but offsetting RSP/R12?
					let fancy_sib = sib.is_some() && (sib != Some(0x24) || rex_x_set);

					if fancy_sib && name != "lea" && name != "nop" { // Only allowed on lea/nop
						continue;
					}

					debug!("REX Byte {:?} on {} {}\n", rex_byte, name, table::bytes(&inst.bytes));

					let used_rex = RefCell::new(false);

					match modrm {
						Some((_, _, _, true)) => *used_rex.borrow_mut() = true,
						_ => (),
					}

					let rex_w = || {
						let r = ext_bit(rex_val, 3, 0) != 0;
						if r {
							*used_rex.borrow_mut() = true;
						}
						r
					};

					let rex_b = || {
						let r = ext_bit(rex_val, 0, 0) != 0;
						if r {
							*used_rex.borrow_mut() = true;
						}
						r
					};

					let modrm = match modrm {
						Some((byte, indir, reg, _)) => {
							let indir = match indir {
								ModRM::Sib(disp, mode) => {

									let sib = sib.unwrap_or(0) as usize;
									let rex_b = ext_bit(rex_val as usize, 0, 3);
									let base = sib & 7 | rex_b;
									let rex_x = ext_bit(rex_val as usize, 1, 3);
									let index = ((sib >> 3) & 7) | rex_x;
									let scale = sib >> 6;

									if rex_b | rex_x != 0 {
										*used_rex.borrow_mut() = true;
									}

									let reg_index = if index == 4 {
										None
									} else {
										Some(index)
									};
									let (reg_base, off) = if mode == 0 && base & 7 == 5 {
										(None, Disp::Imm32)
									} else {
										(Some(base), disp)
									};

									Ok(IndirectAccessFormat {
										base: reg_base,
										index: reg_index,
										scale: 1 << scale,
										disp: off,
									})
								}
								ModRM::Indirect(i) => Ok(i),
								ModRM::Reg(rm) => Err(rm),
							};
							Some((byte, indir, reg))
						}
						_ => None,
					};

					let operand_size_override = prefixes.contains(&P_OP_SIZE);

					let op_size = || if rex_w() { S64 } else { if operand_size_override { S16 } else { S32 } };
					let imm_size = || if op_size() == S16 { S16 } else { S32 };

					let decode_size = |s: Size| {
						match s {
							SRexSize => if rex_w() { S64 } else { S32 },
							SImmSize => imm_size(),
							SOpSize => {

							debug!("Use OpSIZE\n");;
								op_size()},
							_ => s
						}
					};

					let mut effects = Vec::new();

					// TODO: Check for redundant instructions like mov eax, eax

					// TODO: Generate DecodedOperands and use that to check for 8-bit registers with/without REX so REX 40 can be added if required

					let reg_ref = |r: usize, regs: Regs, op_size: Size| {
						OperandFormat::Direct(match regs {
							Regs::GP => RT::GP(r),
							Regs::SSE => RT::SSE(r),
						}, decode_size(op_size))
					};

					macro_rules! get {
						($mem:expr, $reg:expr) => (match modrm {
							Some((_, Ok(mem), other)) => $mem(mem, other),
							Some((_, Err(reg), other)) => $reg(reg, other),
							None => panic!(),
						})
					}

					let modrm_operand = |regs: Regs, op_size: Size| {
						get!(|m, other| (OperandFormat::Indirect(m, decode_size(op_size)), other), |reg, other| (reg_ref(reg, regs, op_size), other))
					};

					let gs = prefixes.contains(&P_SEG_GS);

					let to_mem = |i: IndirectAccessFormat| {
						match i {
							IndirectAccessFormat { base: Some(16), index: None, scale: 0, disp: Disp::None } => Mem::Rip,
							IndirectAccessFormat { base: Some(r), index: None, scale: 1, disp: d } => Mem::Mem(r, d),
							IndirectAccessFormat { base: Some(r), index: None, scale: 0, disp: d } => Mem::Mem(r, d),
							_ => panic!("Unknown form {:?}", i),
						}
					};

					let get_write = || get!(|m, _| if gs { Effect::WriteMem(to_mem(m)) } else { Effect::WriteStack(to_mem(m)) }, |r, _| Effect::ClobReg(r));
					let get_read = || get!(|m, _| if gs { Effect::ReadMem(to_mem(m)) } else { Effect::ReadStack(to_mem(m)) }, |_, _| Effect::None);
					let get_rm = |access| {
						match access {
							Access::Read => get_read(),
							_ => get_write(),
						}
					};

					let mut operands = Vec::new();

					for access in &inst.accesses {
						match *access {
							(reg, Access::ReadWrite) | (reg, Access::Write) => effects.push(Effect::ClobReg(reg)),
							_ => {},
						}
					}

					for op in &inst.operands {
						match *op {
							(Operand::Imm(size), _, access) => {
								effects.push(match decode_size(size) {
									S64 => Effect::Imm64,
									S32 => Effect::Imm32,
									S16 => Effect::Imm16,
									S8 => Effect::Imm8,
									_ => panic!(),
								});
								operands.push((OperandFormat::Imm(decode_size(size)), access));
							}
							(Operand::FixImm(imm, _), _, access) => {
								operands.push((OperandFormat::FixImm(imm), access));
							}
							(Operand::Disp(size), _, access) => {
								match size {
									S8 => effects.push(Effect::Jmp8),
									S32 => effects.push(if name == "call" { Effect::Call32 } else { Effect::Jmp32 }),
									_ => panic!(),
								}
								operands.push((OperandFormat::Disp(decode_size(size)), access));
							}
							(Operand::FixReg(reg, regs), op_size, access) => {
								let r = reg_ref(reg, regs, op_size);
								operands.push((r, access));
								if access != Access::Read && regs == Regs::GP {
									effects.push(Effect::ClobReg(reg));
								}
							}
							(Operand::FixRegRex(mut reg, regs), op_size, access) => {
								if rex_b() {
									reg += 8;
								}
								let r = reg_ref(reg, regs, op_size);
								operands.push((r, access));
								if name == "push" {
									effects.push(Effect::Push(reg));
								} else if name == "pop" {
									effects.push(Effect::Pop(reg));
								} else if access != Access::Read && regs == Regs::GP {
									effects.push(Effect::ClobReg(reg));
								}
							}
							(Operand::Addr, _, access) => {
								effects.push(Effect::CheckAddr);
								operands.push((OperandFormat::IndirectAddr, access));
							}
							(Operand::Rm(regs), op_size, access) => {
								effects.push(if name == "mov" && regs == Regs::GP && !gs {
									get!(|m, o| {
										match access {
											Access::Read => Effect::Load(o, to_mem(m)),
											_ => Effect::Store(to_mem(m), o),
										}
									}, |r, o| Effect::Move(r, o))
								} else {
									get_rm(access)
								});

								let (indir, _) = modrm_operand(regs, op_size);
								operands.push((indir, access));
							}
							// TODO: Ensure that Operand::Rm is present and has the opposite access if Operand::Reg is present
							(Operand::Reg(regs), op_size, access) => { 
								let (_, reg) = modrm_operand(regs, op_size);
								let r = reg_ref(reg, regs, op_size);
								operands.push((r, access));
							}
							(Operand::RmOpcode(_), op_size, access) => {
								if name == "nop" {
									continue;
								}
								effects.push(if name == "call" {
									get!(|m, _| Effect::Call(to_mem(m)), |_, _| Effect::None) // TODO: CFI
								} else {
									get_rm(access)
								});

								let (indir, _) = modrm_operand(Regs::GP, op_size);
								operands.push((indir, access));
							}
							(Operand::Mem(_), op_size, access) => {
								assert!(name == "lea");
								if !rex_w() {
									continue 'inner;
								}
								if fancy_sib {
									effects.push(get!(|_, r| Effect::ClobReg(r), |_, _| panic!()));
								} else {
									effects.push(get!(|m, r| Effect::Lea(r, to_mem(m)), |_, _| panic!()));
								}

								let (indir, _) = modrm_operand(Regs::GP, op_size);
								match indir {
									OperandFormat::Indirect(..) => {
										operands.push((indir, access));
									}
									_ => panic!(),
								}
							}
						}
					}

					if rex_byte.is_some() && !*used_rex.borrow() {
						debug!("Rejected {} {:?}: Unused rex byte\n", table::bytes(prefixes), rex_byte);
						continue;
					}

					let mut bytes = Vec::new();

					bytes.extend_from_slice(prefixes);
					bytes.extend_from_slice(&inst.prefix_bytes);
					rex_byte.map(|r| bytes.push(r));
					bytes.extend_from_slice(&inst.bytes);

					if let Some((b, _, _)) = modrm {
						bytes.push(b as u8);
					}

					if let Some(sib) = sib {
						bytes.push(sib as u8);
					}

					let mut name = inst.name.clone();

					if name == "cwde" && rex_w() { // TODO: Don't cause REX usage
						name = "cdqe".to_string();
					} else if name == "cdq" && rex_w() {
						name = "cqo".to_string();
					}

					let op_size = decode_size(inst.operand_size); // TODO: Don't cause REX usage

					if inst.op_size_postfix {
						name = name.clone() + match op_size {
							S128 => panic!(),
							S64 => "q",
							S32 => "d",
							S16 => "w",
							S8 => "b",
							_ => panic!(),
						}
					}

					debug!("Adding {} ({}) => {:?}\n", name, table::bytes(&bytes), effects);

					let format = InstFormat {
						prefix_bytes: inst.prefix_bytes.clone(),
						bytes: bytes.clone(),
						prefixes: prefixes.to_vec(),
						operands: operands,
						name: name,
						no_mem: inst.no_mem,
						op_size: op_size,
						rex: rex_byte.is_some(),
					};

					cases.push((bytes, effects, format));
				}
			}
		}
	}
}

pub fn parse(cursor: &mut Cursor, disp_off: u64, format: &InstFormat) -> DecodedInst {
	let start = cursor.offset;
	let prefixes = &format.prefixes[..];
	let fs_override = prefixes.contains(&P_SEG_FS);
	let gs_override = prefixes.contains(&P_SEG_GS);

	let has_prefix = |p: u8| {
		if format.prefix_bytes.contains(&p) {
			false
		} else {
			prefixes.contains(&p)
		}
	};

	let mut operands: Vec<(DecodedOperand, Size)> = Vec::new();

	for op in &format.operands {
		match *op {
			(OperandFormat::Imm(size), _) => {
				let imm = read_imm(cursor, size);
				operands.push((DecodedOperand::Imm(imm, size), format.op_size));
			}
			(OperandFormat::FixImm(imm), _) => {
				operands.push((DecodedOperand::Imm(imm, Lit1), Lit1));
			}
			(OperandFormat::Disp(size), _) => {
				let off = read_imm(cursor, size).wrapping_add(disp_off as i64).wrapping_add(cursor.offset as u64 as i64);
				operands.push((DecodedOperand::Imm(off, size), S64));
			}
			(OperandFormat::Direct(reg, op_size), _) => {
				operands.push((DecodedOperand::Direct(reg), op_size));
			}
			(OperandFormat::Indirect(i, op_size), _) => {
				let a = IndirectAccess {
					base: i.base,
					index: i.index,
					scale: i.scale,
					offset: match i.disp {
						Disp::None => 0,
						Disp::Imm8 => read_imm(cursor, S8),
						Disp::Imm32 => read_imm(cursor, S32),
					},
					offset_wide: false,
				};
				operands.push((DecodedOperand::Indirect(a), op_size));
			}
			(OperandFormat::IndirectAddr, _) => {
				let a = IndirectAccess {
					base: None,
					index: None,
					scale: 0,
					offset: read_imm(cursor, S64),
					offset_wide: true,
				};
				operands.push((DecodedOperand::Indirect(a), format.op_size));
			}
		}
	}

	let print_op = |i: usize| {
		let op: (DecodedOperand, Size) = operands[i].clone();

		match op.0 {
			DecodedOperand::Direct(reg) => format!("{}", reg_name(reg, op.1, format.rex)),
			DecodedOperand::Indirect(indir) => {
				let ptr = operand_ptr(format.no_mem, op.1);

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

	let mut prefix = String::new();

	if has_prefix(P_LOCK) {
		prefix.push_str("lock ");
	}

	if has_prefix(P_REP) {
		prefix.push_str("rep ");
	}

	let ops = operands.iter().enumerate().map(|(i, _)| print_op(i)).collect::<Vec<String>>().join(", ");
	let desc = format!("{}{}{}{}", prefix, format.name, if operands.is_empty() { "" } else { " " }, ops);

	DecodedInst {
		operands: operands.clone(),
		desc: desc,
		name: format.name.clone(),
		len: cursor.offset - start + format.bytes.len(),
	}
}
