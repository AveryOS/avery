use table;
use std::cmp;
use effect::{Effect, DecodedOperand, Size, InstFormat, DecodedInst};
use disasm;

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

fn capstone(data: &[u8], disp_off: u64) -> (String, usize) {
	use std::ffi::CStr;
	use std::ptr;
	use capstone::*;

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
			let desc = format!("{} {} (length: {})", mnemonic, ops, (*ci).size).trim().to_string();
			cs_free(ci, count);
			(desc, (*ci).size as usize)
		} else {
			("invalid".to_string(), 1)
		};

		cs_close(&mut handle);

		r
	}
}

pub fn inst(c: &mut Cursor, disp_off: u64, cases: &[(Vec<u8>, Vec<Effect>, InstFormat)]) -> (DecodedInst, Vec<Effect>) {
	let case = cases.iter().find(|i| c.remaining().starts_with(&i.0[..])).unwrap_or_else(|| {
		let data = &c.remaining()[0..cmp::min(16, c.remaining().len())];
		let (desc, len) = capstone(data, 0);
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

pub fn decode(data: &[u8], start: usize, size: usize, disp_off: u64, cases: &[(Vec<u8>, Vec<Effect>, InstFormat)]) {
	let mut targets = Vec::new();
	targets.push(start);

	let mut i = 0;

	while i < targets.len() {
		let mut c = Cursor {
			data: data,
			offset: targets[i],
		};

		println!("disasm:");

		loop {
			let start = c.offset;
			print!("{:#08x}: ", start as u64 + disp_off);
			let (i, effects) = inst(&mut c, disp_off, cases);
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

			println!("{: <40} {:?}", i.desc, effects);

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

			if i.name == "jmp" || i.name == "ret" {
				break
			}
		}

		i += 1;
	}
}
