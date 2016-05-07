use table;
use std::cmp;
use std::ptr;
use effect::{Effect, DecodedOperand, Size, InstFormat, DecodedInst};
use disasm;

pub static mut BRIEF: bool = false;

#[link(name = "capstone", kind = "static")]
extern {}

#[path = "../capstone/capstone.rs"]
#[allow(dead_code, non_snake_case, non_camel_case_types)]
mod capstone;

use self::capstone::csh;

#[derive(Copy, Clone)]
pub struct Cursor<'s> {
	pub data: &'s [u8],
	pub offset: usize,
}

impl<'s> Cursor<'s> {
	pub fn remaining(&self) -> &'s [u8] {
		&self.data[self.offset..]
	}

	pub fn peek(&self) -> u8 {
		self.data[self.offset]
	}

	pub fn next(&mut self) -> u8 {
		let byte = self.data[self.offset];
		self.offset += 1;
		byte
	}
}

pub fn capstone_open() -> csh {
	use self::capstone::*;

	unsafe {
		let mut handle: csh = 0;

		if cs_open(Enum_cs_arch::CS_ARCH_X86, Enum_cs_mode::CS_MODE_64, &mut handle) as u32 != 0 {
			panic!();
		}

		cs_option(handle, Enum_cs_opt_type::CS_OPT_DETAIL, Enum_cs_opt_value::CS_OPT_ON as u64);

		handle
	}
}

pub fn capstone_close(mut handle: csh) {
	use self::capstone::*;

	unsafe {
		cs_close(&mut handle);
	}
}

#[derive(Debug)]
enum Reg {
	IP,
	GP(usize)
}

fn reg(cr: u16) -> Option<Reg> {
	use self::capstone::Enum_x86_reg::*;

	match unsafe { ::std::mem::transmute(cr as u32) } {
		X86_REG_AH | X86_REG_AL | X86_REG_AX | X86_REG_EAX | X86_REG_RAX => Some(Reg::GP(0)),
		X86_REG_CH | X86_REG_CL | X86_REG_CX | X86_REG_ECX | X86_REG_RCX => Some(Reg::GP(1)),
		X86_REG_DH | X86_REG_DL | X86_REG_DX | X86_REG_EDX | X86_REG_RDX => Some(Reg::GP(2)),
		X86_REG_BH | X86_REG_BL | X86_REG_BX | X86_REG_EBX | X86_REG_RBX => Some(Reg::GP(3)),
		X86_REG_SPL | X86_REG_SP | X86_REG_ESP | X86_REG_RSP => Some(Reg::GP(4)),
		X86_REG_BPL | X86_REG_BP | X86_REG_EBP | X86_REG_RBP => Some(Reg::GP(5)),
		X86_REG_SIL | X86_REG_SI | X86_REG_ESI | X86_REG_RSI => Some(Reg::GP(6)),
		X86_REG_DIL | X86_REG_DI | X86_REG_EDI | X86_REG_RDI => Some(Reg::GP(7)),
		X86_REG_R8B | X86_REG_R8W | X86_REG_R8D | X86_REG_R8 => Some(Reg::GP(8)),
		X86_REG_R9B | X86_REG_R9W | X86_REG_R9D | X86_REG_R9 => Some(Reg::GP(9)),
		X86_REG_R10B | X86_REG_R10W | X86_REG_R10D | X86_REG_R10 => Some(Reg::GP(10)),
		X86_REG_R11B | X86_REG_R11W | X86_REG_R11D | X86_REG_R11 => Some(Reg::GP(11)),
		X86_REG_R12B | X86_REG_R12W | X86_REG_R12D | X86_REG_R12 => Some(Reg::GP(12)),
		X86_REG_R13B | X86_REG_R13W | X86_REG_R13D | X86_REG_R13 => Some(Reg::GP(13)),
		X86_REG_R14B | X86_REG_R14W | X86_REG_R14D | X86_REG_R14 => Some(Reg::GP(14)),
		X86_REG_R15B | X86_REG_R15W | X86_REG_R15D | X86_REG_R15 => Some(Reg::GP(15)),
		X86_REG_RIP => Some(Reg::IP),
		X86_REG_EFLAGS => None,
		_ => panic!("Unknown register {}", cr),
	}
}

pub fn capstone(handle: &mut csh, data: &[u8], disp_off: u64, inst: &DecodedInst, effects: &[Effect]) -> bool {
	use std::ffi::CStr;
	use self::capstone::*;

	let mut error = false;

	unsafe {
		let mut ci: *mut cs_insn = ptr::null_mut();

		let count = cs_disasm(*handle, data.as_ptr(), data.len() as u64, disp_off, 0, &mut ci);

		if count > 0 {
			let mnemonic = CStr::from_ptr((*ci).mnemonic[..].as_ptr()).to_str().unwrap();
			let ops = CStr::from_ptr((*ci).op_str[..].as_ptr()).to_str().unwrap();
			let desc = format!("{} {}", mnemonic, ops).trim().to_string();

			let detail = &mut *(*ci).detail;

			let reads: Vec<_> = detail.regs_read[0..(detail.regs_read_count as usize)].iter().filter_map(|&r| reg(r)).collect();
			let writes: Vec<_> = detail.regs_write[0..(detail.regs_write_count as usize)].iter().filter_map(|&r| reg(r)).collect();

			let x86 = &*detail.x86();

			for op in &x86.operands[0..(x86.op_count as usize)] {

			}

			//println!("on {} {} - reads: {:?} - writes: {:?} - effects {:?}", table::bytes(data), desc, reads, writes, effects);


			if inst.desc != desc {
				println!("on {}\n  c: {}\n  m: {}", table::bytes(data), desc, inst.desc);
				error = true;
			}

			if (*ci).size as usize != inst.len {
				println!("on {}\n  {}\n  len(c): {}\n  len(m): {}", table::bytes(data),  desc, (*ci).size, inst.len);
				error = true;
			}

			cs_free(ci, count);
		} else {
			println!("on {}\n  c: invalid\n  m: {}", table::bytes(data), inst.desc);
			error = true;
			//println!("  inst {:?}", inst);
		}
	}

	error
}

pub fn capstone_simple(data: &[u8], disp_off: u64) -> Option<(String, usize)> {
	use std::ffi::CStr;
	use std::ptr;
	use self::capstone::*;

	unsafe {
		let mut handle: csh = 0;

		if cs_open(Enum_cs_arch::CS_ARCH_X86, Enum_cs_mode::CS_MODE_64, &mut handle) as u32 != 0 {
			panic!();
		}

		cs_option(handle, Enum_cs_opt_type::CS_OPT_DETAIL, Enum_cs_opt_value::CS_OPT_ON as u64);

		let mut ci: *mut cs_insn = ptr::null_mut();

		let count = cs_disasm(handle, data.as_ptr(), data.len() as u64, disp_off, 0, &mut ci);

		let r = if count > 0 {
			let mnemonic = CStr::from_ptr((*ci).mnemonic[..].as_ptr()).to_str().unwrap();
			let ops = CStr::from_ptr((*ci).op_str[..].as_ptr()).to_str().unwrap();
			let desc = format!("{} {}", mnemonic, ops).trim().to_string();
			cs_free(ci, count);
			Some((desc, (*ci).size as usize))
		} else {
			None
		};

		cs_close(&mut handle);

		r
	}
}

pub fn inst(c: &mut Cursor, disp_off: u64, cases: &[(Vec<u8>, Vec<Effect>, InstFormat)]) -> (DecodedInst, Vec<Effect>) {
	let case = cases.iter().find(|i| c.remaining().starts_with(&i.0[..])).unwrap_or_else(|| {
		let data = &c.remaining()[0..cmp::min(16, c.remaining().len())];
		let (desc, len) = capstone_simple(data, 0).unwrap_or(("invalid".to_string(), 1));
		let bytes = table::bytes(&c.remaining()[0..len]);

		println!("unknown |{}| capstone: {}", bytes, desc);
		panic!("unknown |{}| capstone: {}", bytes, desc);
	});

	c.offset += case.2.bytes.len();

	let inst = disasm::parse(c, disp_off, &case.2);
	(inst, case.1.clone())
}

pub fn find_effect(cursor: &Cursor, cases: &[(Vec<u8>, Vec<Effect>)]) -> Option<Vec<Effect>> {
	cases.iter().find(|i| cursor.remaining().starts_with(&i.0[..])).map(|i| i.1.clone())
}

pub fn decode(data: &[u8], func_start: usize, size: usize, disp_off: u64, cases: &[(Vec<u8>, Vec<Effect>, InstFormat)]) {
	let mut targets = Vec::new();
	let mut cp = capstone_open();
	targets.push(func_start);

	let mut i = 0;

	while i < targets.len() {
		let mut c = Cursor {
			data: data,
			offset: targets[i],
		};

		println!("disasm:");

		loop {
			let start = c.offset;
			let address = start as u64 + disp_off;
			print!("{:#08x}: ", address);
			let cs_data = &c.remaining()[0..cmp::min(16, c.remaining().len())];
			let (i, effects) = inst(&mut c, address, cases);
			let mut str = String::new();

			let byte_print_len = cmp::min(8, i.len);

			for b in c.data[start..(start + byte_print_len)].iter() {
				str.push_str(&format!("{:02x}", b));
			}

			for _ in 0..(8 - byte_print_len) {
				str.push_str("  ");
			}
			str.push_str(" ");

			print!("{}", str);

			println!("{: <40} {:?} ({:x}/{:x})", i.desc, effects, c.offset - func_start, size);
/*
			if capstone(&mut cp, cs_data, address, &i, &effects) {
				panic!("Capstone output didn't match");
			}
*/
			if effects.iter().any(|o| match *o { Effect::Jmp32 | Effect::Jmp8 => true, _ => false }) {
				let op: (DecodedOperand, Size) = i.operands.first().unwrap().clone();
				let off = match op.0 {
					DecodedOperand::Imm(off, _) => {
						Some(off as u64)
					}
					_ => None,
				};
				if let Some(off) = off {
					let off = off as usize;
					if off >= start && off < start + size {
						if let Err(i) = targets.binary_search(&off) {
							targets.insert(i, off);
						}
					} else {
						//println!("Jump outside of symbol {:#x}", off);
					}
				}
			}

			if i.name == "jmp" || i.name == "ret" || i.name == "ud2" {
				break
			}

			if c.offset - func_start >= size {
				println!("ERROR: Instruction went outside function");
				panic!("Instruction went outside function");
			}
		}

		i += 1;
	}

	capstone_close(cp);
}
