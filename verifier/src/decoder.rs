use table;
use std::cmp;
use effect::{DecodedOperand, Operand, Inst, Size};
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

fn prefixes<'s>(c: &mut Cursor<'s>) -> &'s [u8] {
	let mut prefixes = Vec::new();
	let s = c.offset;
	for _ in 0..3 {
		let byte = c.peek();
		if table::ALL_PREFIXES.contains(&byte) { 
			if prefixes.contains(&byte) {
				break
			}
			c.next();
			prefixes.push(byte);
		} else {
			break
		}
	}
	&c.data[s..c.offset]
}

pub fn inst(c: &mut Cursor, disp_off: u64, insts: &[Inst]) -> Inst {
	let start = c.offset;
	let mut c_old = c.clone();
	let pres = prefixes(c);
	let rex = c.peek();
	let rex = match rex {
		0x40...0x4F => {
			c.next();
			Some(rex)
		}
		_ => None
	};
	let inst = disasm::parse(c, rex, pres, disp_off, insts);

	let print_debug = |c: &mut Cursor| {
		unsafe { disasm::DEBUG = true };
		disasm::parse(c, rex, pres, disp_off, insts);
	};

	let mut inst = inst.unwrap_or_else(|| {
		print_debug(&mut c_old);
		panic!("on |{}| unknown opcode {:x}", table::bytes(c_old.remaining()), c.next());
	});

	inst.len = c.offset - start;
	inst
}

pub fn decode(data: &[u8], start: usize, size: usize, disp_off: u64, insts: &[Inst]) {
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
			let i = inst(&mut c, disp_off, insts);
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

			println!("{: <40} {}", i.desc, format!("{}", format!("{:?}", i)));

			if i.operands.iter().any(|o| match *o { (Operand::Disp(..), _) => true, _ => false }) {
				let op: (DecodedOperand, Size) = i.decoded_operands.first().unwrap().clone();
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
