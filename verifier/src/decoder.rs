use table;
use std::cmp;
use std::fs::File;
use std::sync::Mutex;
use std::sync::atomic::{Ordering, AtomicBool};

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

fn ud(c: &mut Cursor, disp_off: u64) -> (String, usize) {
	use std::process::Command;
	use std::process::Stdio;
	use std::io::Write;
	let mut ud = Command::new("udis86/install/bin/udcli")
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
			let c_old = c.clone();
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

			let print_debug = || {
				unsafe { table::DEBUG = true };
				inst(&mut c_old.clone(), disp_off);
			};

			let i = i.unwrap_or_else(|| {
				print_debug();
				panic!("unknown opcode {:x} (ud: {})", c.next(), ud_str)
			});

			println!("{}", i.desc);

			if !ud2_match(&ud_str, &i) {
				print_debug();
				panic!("udis86 output didn't match |{}|", ud_str);
			}

			if ud_len != c.offset - start {
				print_debug();
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

fn decode_testrp(xs: &[u8], pre: &[u8], rex: &[u8], f: &Mutex<File>) {
	let mut s = Vec::new();
	s.extend(pre);
	s.extend(rex);
	s.extend(xs);
	decode_test(&s, f);
}

fn decode_testp(xs: &[u8], pre: &[u8], f: &Mutex<File>) {
	decode_testrp(xs, pre, &[], f);
	for b in 0x40..0x4f {
		decode_testrp(xs, pre, &[b], f);
	}
}

pub fn decode_test_allp(xs: &[u8], f: &Mutex<File>) {
	decode_testp(xs, &[], f);
	decode_testp(xs, &[0x64], f);
	decode_testp(xs, &[0x65], f);
	decode_testp(xs, &[0xF0], f);
	decode_testp(xs, &[0x66], f);
	decode_testp(xs, &[0x66, 0xF0], f);
	decode_testp(xs, &[0x65, 0xF0], f);
	decode_testp(xs, &[0x64, 0xF0], f);
	decode_testp(xs, &[0x64, 0x66], f);
	decode_testp(xs, &[0x65, 0x66], f);
	decode_testp(xs, &[0x64, 0x65], f);
	decode_testp(xs, &[0x64, 0x65, 0xF0], f);
	decode_testp(xs, &[0x64, 0x65, 0x66], f);
	decode_testp(xs, &[0x66, 0x65, 0xF0], f);
	decode_testp(xs, &[0x64, 0x66, 0xF0], f);
	decode_testp(xs, &[0x64, 0x66, 0x65, 0xF0], f);
}

fn ud2_match(ud: &str, inst: &table::Instruction) -> bool {
	use table::*;
	use table::Size::*;
	if (inst.name == "out" && inst.ops[1].1 == S32) ||
	   (inst.name == "in"  && inst.ops[0].1 == S32) {
		return true;
	}
	if inst.name == "movsxd" && inst.ops[0].1 == S32 {
		return true;
	}
	ud == inst.desc
}

pub static FOUND_ERRORS: AtomicBool = AtomicBool::new(false);

pub fn decode_test(xs: &[u8], f: &Mutex<File>) {
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
		if unsafe { table::DEBUG } {
			println!("Decoded {} = {}\n{:?}", table::bytes(&xs[..]), i.desc, i);
		}
		if !ud2_match(&ud_str, &i) {
			FOUND_ERRORS.store(true, Ordering::SeqCst);
			writeln!(f.lock().unwrap(), "{}", table::bytes(xs)).unwrap();
			println!("On: {}; len:{} |{}|; udis86 output didn't match len:{} |{}|", str, c.offset, i.desc, ud_len, ud_str);
			writeln!(&mut stderr(), "On: {}; len:{} |{}|; udis86 output didn't match len:{} |{}|", str, c.offset, i.desc, ud_len, ud_str).unwrap();
		} else if ud_len != c.offset {
			FOUND_ERRORS.store(true, Ordering::SeqCst);
			writeln!(f.lock().unwrap(), "{}", table::bytes(xs)).unwrap();
			println!("On: {}; Instruction was of length {}, while udis86 was length {}", str, c.offset, ud_len);
			writeln!(&mut stderr(), "On: {}; Instruction was of length {}, while udis86 was length {}", str, c.offset, ud_len).unwrap();
		}
	} else {
		if unsafe { table::DEBUG } {
			println!("No decoding for {}", table::bytes(&xs[..]));
		}
	}
}
