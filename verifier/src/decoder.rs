use table;

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
						 .stdin(Stdio::piped())
						 .stdout(Stdio::piped())
						 .spawn().unwrap();
	ud.stdin.as_mut().unwrap().write(&c.data[c.offset..(c.offset + 16)]).unwrap();
	let dis = ud.wait_with_output().unwrap().stdout;
	let str = String::from_utf8_lossy(&dis);
	let l = &str.lines().next().unwrap()[16..].trim();
	let mut ws = l.split_whitespace();
	//ws.next().unwrap();
	let bs = ws.next().unwrap();
	(l.to_string(), bs.len() / 2)
}

fn inst(c: &mut Cursor) -> (table::Instruction, usize, String) {
	let (ud_str, ud_len) = ud(c);
	let mut s = String::new();
	prefixes(c);
	let rex = c.peek();
	let rex = match rex {
		0x40...0x4F => {
			s.push_str(&format!("rex: {:x}", rex));
			c.next();
			Some(rex)
		}
		_ => None
	};
	match table::parse(c, rex) {
		Some(t) => (t, ud_len, ud_str),
		None => panic!("unknown opcode {:x} (ud: {})", c.next(), ud_str),
	}
}

pub fn decode(data: &[u8], offset: usize) {
	let mut c = Cursor {
		data: &data[offset..],
		offset: 0,
	};

	while offset < data.len() {
		let start = c.offset;
		print!("{:#08x}: ", start);
		let (i, ud_len, ud_str) = inst(&mut c);
		let mut str = String::new();

		for b in c.data[start..c.offset].iter() {
			str.push_str(&format!("{:02x}", b));
		}

		for _ in 0..(8 - (c.offset - start)) {
			str.push_str("  ");
		}
		str.push_str(" ");

		str.push_str(&i.desc);

		println!("L:{}\n{:#08x}: U:{}", str, start, ud_str);

		if ud_str != str {
			panic!("udis86 output didn't match");
		}

		if ud_len != c.offset - start {
			panic!("Instruction was of length {}, while udis86 was length {}", c.offset - start, ud_len);
		}

		if i.terminating {
			break
		}
	}
}