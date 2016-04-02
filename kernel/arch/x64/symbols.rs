use arch::dwarf;

use elfloader::{self, ElfBinary, elf};

pub struct Demangle<'a>(pub &'a str);
impl<'a> ::core::fmt::Display for Demangle<'a> {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		if &self.0[..3] == "_ZN" {
			let mut s = &self.0[3..];
			while !s.is_empty()
			{
				let n = s.read_num();
				if n == 0 { break ; }
				try!(write!(f, "::{}", &s[..n]));
				s = &s[n..];
			}
			write!(f, "::{}", s)
		}
		else {
			write!(f, "{}", self.0)
		}
	}
}
trait ReadInt {
	fn read_num(&mut self) -> usize;
}
impl<'a> ReadInt for &'a str {
	fn read_num(&mut self) -> usize {
		let mut b = self.as_bytes();
		let mut rv = 0;
		while let Some(v) = (b[0] as char).to_digit(10) {
			rv *= 10;
			rv += v as usize;
			b = &b[1..];
		}
		// SAFE: Validity is maintained
		*self = unsafe { ::core::str::from_utf8_unchecked(b) };
		rv
	}
}

fn get_symbol_for_addr<'s>(addr: usize, from_ip: bool, bin: &'s ElfBinary<'s>) -> Option<&'s elf::Symbol> {
	bin.find_symbol(|sym| {
		if sym.sym_type() != elf::STT_FUNC {
			return false;
		}

		if from_ip {
			// We are looking up a RIP address, do an exact match
			if usize::coerce(sym.value) <= addr && usize::coerce(sym.value + sym.size) > addr {
				return true;
			}
		} else {
			// We are looking up a return address from the stack
			// In this case the address after the end of the symbol
			// actually means that we called from inside the symbol.
			// The entry point of the function would also belong to
			// the previous code.
			if usize::coerce(sym.value) < addr && usize::coerce(sym.value + sym.size) >= addr {
				return true;
			}
		}
		false
	})
}

/// Obtain the old RBP value and return address from a provided RBP value
pub fn backtrace(bp: usize) -> Option<(usize,usize)> {
	use std::mem::size_of;

	if bp == 0 {
		return None;
	}
	unsafe {
		let ptr: *const [usize; 2] = ::core::mem::transmute(bp);
		let newbp = (*ptr)[0];
		let newip = (*ptr)[1];
		Some((newbp, newip))
	}
}


/// Print a backtrace, starting at the current location.
pub fn print_backtrace()
{
	let cur_bp: usize;
	// SAFE: Reads from bp
	unsafe{ asm!("mov %rbp, $0" : "=r" (cur_bp)); }
	print!("Backtrace:\n{}", Backtrace(cur_bp as usize, None, None));
}
pub struct Backtrace<'s>(pub usize, pub Option<usize>, pub Option<&'s ElfBinary<'s>>);
impl<'s> ::core::fmt::Display for Backtrace<'s> {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		let mut stack = Vec::new();
		let info = self.2.and_then(|b| dwarf::get_dwarf_info_from_elf(b));
		if let Some(ip) = self.1 {
			stack.push(dwarf::Bound::new(ip, false));
		}
		let mut bp = self.0;
		while let Option::Some((newbp, ip)) = backtrace(bp) {
			stack.push(dwarf::Bound::new(ip, true));
			bp = newbp;
		}
		info.map(|info| dwarf::parse_line_units(&info, &mut stack).unwrap());

		for entry in stack {
			let (name, offset) = self.2.and_then(|bin| {
				get_symbol_for_addr(usize::coerce(entry.target), !entry.return_target, bin)
					.map(|s| (bin.symbol_name(s), entry.target - s.value))
			}).unwrap_or(("<unknown>", 0));
			try!(write!(f, " {:#x} - {}+{:#x} ({}/{}:{}M@{:#x})\n", entry.target, Demangle(name), offset, entry.dir, entry.file, entry.line, entry.address));
		}

		Ok( () )
	}
}
