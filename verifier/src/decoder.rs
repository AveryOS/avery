use table;
use std::cmp;

#[derive(Copy, Clone)]
pub struct Cursor<'s> {
	pub data: &'s [u8],
	pub offset: usize,
}

impl<'s> Cursor<'s> {
	pub fn peek(&self) -> u8 {
		self.data[self.offset]
	}

	pub fn next(&mut self) -> u8 {
		let byte = self.data[self.offset];
		self.offset += 1;
		byte
	}
}

fn prefixes(c: &mut Cursor) -> Vec<u8> {
	let mut prefixes = Vec::new();
	for _ in 0..3 {
		let byte = c.peek();
		match byte {
			0x66 | 0x67 | 0x2E | 0x3E | 0x26 | 0x64 | 0x65 | 0x36 | 0xF0 | 0xF2 | 0xF3 => { 
				if prefixes.contains(&byte) {
					break
				}
				c.next();
				prefixes.push(byte);
			}
			_ => break
		}
	}
	prefixes
}

fn ud(c: &mut Cursor) -> (String, usize) {
	use std::process::Command;
	use std::process::Stdio;
	use std::io::Write;

	let mut ud = Command::new("udcli")
						 .arg("-64")
						 .arg("-o")
						 .arg(format!("{:x}", c.offset))
						 .arg("-noff")
						 .arg("-nohex")
						 .stdin(Stdio::piped())
						 .stdout(Stdio::piped())
						 .spawn().unwrap();
	ud.stdin.as_mut().unwrap().write(&c.data[c.offset..(c.offset + 16)]).unwrap();
	let dis = ud.wait_with_output().unwrap().stdout;
	let str = String::from_utf8_lossy(&dis);
	let l = &str.lines().next().unwrap();
	let mut ws = l.split_whitespace();
	let len = ws.next().unwrap();
	(l[len.len()..].trim().to_string(), len.parse().unwrap())
}

fn inst(c: &mut Cursor) -> (table::Instruction, usize, String) {
	let (ud_str, ud_len) = ud(c);
	let mut s = String::new();
	let pres = prefixes(c);
	let rex = c.peek();
	let rex = match rex {
		0x40...0x4F => {
			s.push_str(&format!("rex: {:x}", rex));
			c.next();
			Some(rex)
		}
		_ => None
	};
	match table::parse(c, rex, &pres) {
		Some(t) => (t, ud_len, ud_str),
		None => panic!("unknown opcode {:x} (ud: {})", c.next(), ud_str),
	}
}

pub fn decode(data: &[u8], offset: usize) {
	let mut targets = Vec::new();
	targets.push(offset);

	let mut i = 0;

	while i < targets.len() {
		let mut c = Cursor {
			data: data,
			offset: targets[i],
		};

		println!("disasm:");

		loop {
			let start = c.offset;
			print!("{:#08x}: ", start);
			let (i, ud_len, ud_str) = inst(&mut c);
			let mut str = String::new();

			let byte_print_len = cmp::min(8, c.offset - start);

			for b in c.data[start..(start + byte_print_len)].iter() {
				str.push_str(&format!("{:02x}", b));
			}

			for _ in 0..(8 - byte_print_len) {
				str.push_str("  ");
			}
			str.push_str(" ");

			str.push_str(&i.desc);

			println!("{}", str);

			if ud_str != i.desc {
				panic!("udis86 output didn't match {}", ud_str);
			}

			if ud_len != c.offset - start {
				panic!("Instruction was of length {}, while udis86 was length {}", c.offset - start, ud_len);
			}

			if i.branch {
				let op = i.ops.first().unwrap().clone();
				let off = match op.0 {
					table::Operand::Imm(off) => {
						Some(off.0 as u64)
					}
					_ => None,
				};
				if let Some(off) = off {
					let off = off as usize;
					if let Err(i) = targets.binary_search(&off) {
						targets.insert(i, off);
					}
				}
			}

			if i.terminating {
				break
			}
		}

		i += 1;
	}
}