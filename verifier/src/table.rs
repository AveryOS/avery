use decoder::Cursor;

trace_macros!(true);

enum Registers {
	RAX, RCX, RDX, RBX, RSP, RBP, RSI, RDI,
          R8, R9, R10, R11, R12, R13, R14, R15
}

fn ext_bit(b: usize, i: usize, t: usize) -> usize {
	((b >> i) & 1) << t
}

pub fn parse(c: &mut Cursor, rex: Option<u8>) -> Option<String> {
	let mut r = String::new();
	let rex = rex.unwrap_or(0);
	let src;
	let dst;
	let regs = &["rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi",
          "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15"];

	macro_rules! modrm {
		() => ({
			let modrm = c.next() as usize;
			let mode = modrm >> 6;
			let reg = ((modrm >> 3) & 7) | ext_bit(rex as usize, 2, 3);
			let rm = modrm & 7 | ext_bit(rex as usize, 0, 3);

			let name = if rm & 7 == 4 { // SIB BYTE
				"sib"
			} else {
				if rm & 7 == 5 { // RIP relative
					"rip"
				} else {
					regs[rm]
				}
			};

			let off = match mode {
				1 => {
					c.next() as i8 as i64
				}
				2 => {
					let mut v = c.next() as u32;
					v |= (c.next() as u32) << 8;
					v |= (c.next() as u32) << 8;
					v |= (c.next() as u32) << 8;
					v as i32 as i64
				}
				_ => 0,
			};

			let indir = if off != 0 {
				format!("[{}+{:#x}]", name, off)
			} else {
				format!("[{}]", name)
			};

			(indir, reg)
		})
	}

	macro_rules! regrm {
		() => ({
			let (indir, reg) = modrm!();
			let dir = format!("{}", regs[reg]);
			(dir, indir)
		})
	}

	macro_rules! opts {
		(reg, rm $($rest:tt)*) => ({
			log_syntax!(OPT reg_rm);
			let (dir, indir) = regrm!();
			dst = dir;
			src = indir;
			comma_opts!($($rest)*);
		});
		(rm, reg $($rest:tt)*) => ({
			log_syntax!(OPT rm_reg);
			let (dir, indir) = regrm!();
			dst = indir;
			src = dir;
			comma_opts!($($rest)*);
		});
	}

	macro_rules! comma_opts {
		() => ();
		(,$($rest:tt)*) => ({
			opts!($($rest)*);
		});
	}

	macro_rules! op {
		($code:expr, $name:expr, $($arg:tt)*) => ({
			if c.peek() == $code {
				c.next();
				r.push_str($name);
				opts!($($arg)*);
				return Some(format!("{} {}, {}", r, dst, src));
			}
		})
	}

	macro_rules! pair {
		($code:expr, $name:expr, $($arg:tt)*) => ({
			op!($code, $name, $($arg)*);
			op!(($code + 1), $name, $($arg)*);
		})
	}

	pair!(0x88, "mov", rm, reg);
	pair!(0x8a, "mov", reg, rm);
/*
	for reg in 0..8 {
		op!(0xb0 + reg, "mov", rm);
	}
*/
	None
}