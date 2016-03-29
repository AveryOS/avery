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

pub fn get_symbol_info_for_addr(addr: usize) -> Option<(&'static str, usize, &'static str, usize, usize)> {
	return None;
	let bound = dwarf::parse_line_units(&dwarf::get_dwarf_info(), addr).unwrap();

	let sym = "unknown";// dwarf::parse_info_units(&dwarf::get_dwarf_info(), addr as u64).unwrap().unwrap_or("<unknown>");

	//let (sym, start_addr) = get_symbol_for_addr(addr).unwrap_or(("unknown", 0));

	Some((bound.name, bound.line as usize, sym, 0, usize::coerce(bound.address)))
}

fn get_symbol_info_for_addr2<'s>(addr: usize, bin: &'s ElfBinary<'s>) -> Option<(&'s str, usize, &'s str, usize, usize)> {
	let result = bin.find_symbol(|sym| {
		if sym.sym_type() != elf::STT_FUNC {
			return false;
		}
		if usize::coerce(sym.value) <= addr &&  usize::coerce(sym.value + sym.size) > addr {
			return true;
		}
		false
	});
	result.map(|s| {
		("?", 1, bin.symbol_name(s), addr - usize::coerce(s.value), 0)
	} )
}

/// Obtain the old RBP value and return address from a provided RBP value
pub fn backtrace(bp: usize) -> Option<(usize,usize)>
{
	use std::mem::size_of;

	if bp == 0 {
		return None;
	}
	if bp % size_of::<usize>() != 0 {
		return None;
	}
	/*if ! ::memory::buf_valid(bp as *const (), 16) {
		return None;
	}*/

	// [rbp] = oldrbp, [rbp+8] = IP
	// SAFE: Pointer access checked, any alias is benign
	unsafe
	{
		let ptr: *const [usize; 2] = ::core::mem::transmute(bp);
		if false  /* ! ::arch::memory::virt::is_reserved(ptr)*/ {
			None
		}
		else {
			let newbp = (*ptr)[0];
			let newip = (*ptr)[1];
			// Check validity of output BP, must be > old BP (upwards on the stack)
			// - If not, return 0 (which will cause a break next loop)
			if newbp <= bp {
				Some( (0, newip) )
			}
			else {
				Some( (newbp, newip) )
			}
		}
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
		let mut bp = self.0;
		let mut ip = self.1;
		while let Option::Some((newbp, ip)) = {
			if let Some(i) = ip {
				ip = None;
				Some((bp, i))
			} else {
				backtrace(bp)
			}
		} {
			try!(write!(f, " {:#x}", ip));
			let info = match self.2 {
				Some(elf) => get_symbol_info_for_addr2(usize::coerce(ip), elf),
				None => get_symbol_info_for_addr(usize::coerce(ip) - 1)
			};
			if let Some( (file, line, name, ofs, mofs) ) = info {
				try!(write!(f, " - {}+{:#x} ({}:{}M@{:#x})", Demangle(name), ofs, file, line, mofs));
			}
			try!(write!(f, "\n"));
			bp = newbp;
		}
		Ok( () )
	}
}
