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

pub fn inst(c: &mut Cursor, disp_off: u64, cases: &[(Vec<u8>, Vec<Effect>, InstFormat)]) -> (DecodedInst, Vec<Effect>) {
	let start = c.offset;
	let case = cases.iter().find(|i| c.remaining().starts_with(&i.0[..])).unwrap();

	c.offset += case.2.bytes.len();

	let mut c_old = c.clone();

	let inst = disasm::parse(c, disp_off, &case.2);

	let print_debug = |c: &mut Cursor| {
		unsafe { disasm::DEBUG = true };
		disasm::parse(c, disp_off, &case.2);
	};

	let mut inst = inst.unwrap_or_else(|| {
		print_debug(&mut c_old);
		panic!("on |{}| unknown opcode {:x}", table::bytes(c_old.remaining()), c.next());
	});

	inst.len = c.offset - start;
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
