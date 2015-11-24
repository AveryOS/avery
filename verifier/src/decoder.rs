use table;
use std::cmp;

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
	&c.data[s..c.offset]
}

fn ud(c: &mut Cursor, disp_off: u64) -> (String, usize) {
	use std::process::Command;
	use std::process::Stdio;
	use std::io::Write;

	let mut ud = Command::new("udcli")
						 .arg("-64")
						 .arg("-o")
						 .arg(format!("{:x}", c.offset as u64 + disp_off))
						 .arg("-noff")
						 .arg("-nohex")
						 .stdin(Stdio::piped())
						 .stdout(Stdio::piped())
						 .spawn().unwrap();
	ud.stdin.as_mut().unwrap().write(&c.data[c.offset..(c.offset + 16)]).unwrap();
	let dis = ud.wait_with_output().unwrap().stdout;
	let str = String::from_utf8_lossy(&dis);
	//println!("ud:{}",str);
	let l = &str.lines().next().unwrap();
	let mut ws = l.split_whitespace();
	let len = ws.next().unwrap();
	(l[len.len()..].trim().to_string(), len.parse().unwrap())
}

pub fn inst(c: &mut Cursor, disp_off: u64) -> (Option<table::Instruction>, usize, String) {
	let (ud_str, ud_len) = ud(c, disp_off);
	let pres = prefixes(c);
	let rex = c.peek();
	let rex = match rex {
		0x40...0x4F => {
			c.next();
			Some(rex)
		}
		_ => None
	};
	(table::parse(c, rex, pres, disp_off), ud_len, ud_str)
}

pub fn decode(data: &[u8], start: usize, size: usize, disp_off: u64) {
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
			let (i, ud_len, ud_str) = inst(&mut c, disp_off);
			let mut str = String::new();

			let byte_print_len = cmp::min(8, ud_len);

			for b in c.data[start..(start + byte_print_len)].iter() {
				str.push_str(&format!("{:02x}", b));
			}

			for _ in 0..(8 - byte_print_len) {
				str.push_str("  ");
			}
			str.push_str(" ");

			print!("{}", str);

			let i = i.unwrap_or_else(|| panic!("unknown opcode {:x} (ud: {})", c.next(), ud_str));

			println!("{}", i.desc);

			if ud_str != i.desc {
				panic!("udis86 output didn't match |{}|", ud_str);
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
					if off >= start && off < start + size {
						if let Err(i) = targets.binary_search(&off) {
							targets.insert(i, off);
						}
					} else {
						//println!("Jump outside of symbol {:#x}", off);
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

fn decode_testrp(xs: &[u8], pre: &[u8], rex: &[u8]) {
	let mut s = Vec::new();
	s.extend(pre);
	s.extend(rex);
	s.extend(xs);
	decode_test(&s);
}

fn decode_testp(xs: &[u8], pre: &[u8]) {
	decode_testrp(xs, pre, &[]);
	for b in 0x40..0x4f {
		decode_testrp(xs, pre, &[b]);
	}
}

pub fn decode_test_allp(xs: &[u8]) {
	decode_test(xs);
	decode_testp(xs, &[0x64]);
	decode_testp(xs, &[0x65]);
	decode_testp(xs, &[0xF0]);
	decode_testp(xs, &[0x66]);
	decode_testp(xs, &[0x66, 0xF0]);
	decode_testp(xs, &[0x65, 0xF0]);
	decode_testp(xs, &[0x64, 0xF0]);
	decode_testp(xs, &[0x64, 0x66]);
	decode_testp(xs, &[0x65, 0x66]);
	decode_testp(xs, &[0x64, 0x65]);
	decode_testp(xs, &[0x64, 0x65, 0xF0]);
	decode_testp(xs, &[0x64, 0x65, 0x66]);
	decode_testp(xs, &[0x66, 0x65, 0xF0]);
	decode_testp(xs, &[0x64, 0x66, 0xF0]);
	decode_testp(xs, &[0x64, 0x66, 0x65, 0xF0]);
}

pub fn decode_test(xs: &[u8]) {
	use std::io::stderr;
	use std::io::Write;

	let mut c = Cursor {
		data: &xs[..],
		offset: 0,
	};
	let (i, ud_len, ud_str) = inst(&mut c, 0);

	if let Some(i) = i {
		let mut str = String::new();
		for b in xs[0..ud_len].iter() {
			str.push_str(&format!("{:02x}", b));
		}
		//println!("testing {:?} = {}", &xs[..], i.desc);
		if ud_str != i.desc {
			println!("On: {}; len:{} |{}|; udis86 output didn't match len:{} |{}|", str, c.offset, i.desc, ud_len, ud_str);
			writeln!(&mut stderr(), "On: {}; len:{} |{}|; udis86 output didn't match len:{} |{}|", str, c.offset, i.desc, ud_len, ud_str).unwrap();
		} else if ud_len != c.offset {
			println!("On: {}; Instruction was of length {}, while udis86 was length {}", str, c.offset, ud_len);
			writeln!(&mut stderr(), "On: {}; Instruction was of length {}, while udis86 was length {}", str, c.offset, ud_len).unwrap();
		}
	}
}

#[cfg(test)]
mod tests {
	use decoder::*;
	use std::io::Write;

	#[test]
	fn cases() {
		println!("cases...");
		decode_n(&[0x64, 0x26]);
	}
/*
	//#[quickcheck]
	fn decode_q(xs: Vec<u8>) -> bool {
		decode_n(&xs);
		true
	}
*/
	fn decode_n(xs: &[u8]) {
		let mut v = Vec::new();
		v.extend(xs);
		while v.len() < 16 {
			v.push(0);
		}
		decode_test(&v)
	}

	#[test]
	fn decode_all() {
		println!("starting decode_all...");
		::std::io::stdout().flush().ok();
		let mut xs = Vec::new();
		while xs.len() < 16 {
			xs.push(0);
		}
		while *xs.last().unwrap() != 255 {
			decode_test(&xs);

			for j in 0..xs.len() {
				if xs[j] == 255 {
					xs[j] = 0;
				} else {
					xs[j] += 1;
					break;
				}
			}
		}
		decode_test(&xs);
	}
}