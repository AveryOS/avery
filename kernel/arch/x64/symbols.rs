use arch::dwarf;

#[repr(C)]
pub struct Elf32_Sym {
	st_name: u32,
	st_value: u32,
	st_size: u32,
	st_info: u8,
	st_other: u8,
	st_shndx: u16,
}

struct State {
	symtab: &'static [Elf32_Sym],
	strtab: &'static [u8],
	/// Symbol address offset - used for AMD64 where the symbol table has truncated symbols
	addr_offset: usize,
}

static mut S_SYMS: State = State { symtab: &[], strtab: &[], addr_offset: 0 };

/// UNSAFE: Should only ever be called once, and before multithreading
pub unsafe fn set_symtab(symtab: &'static [Elf32_Sym], strtab: &'static [u8], offset: usize) {
	assert!(S_SYMS.symtab.len() == 0, "Setting symbol table twice");
	S_SYMS = State {
		symtab: symtab,
		strtab: strtab,
		addr_offset: offset,
		};
}

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
	let bound = dwarf::parse_line_units(&dwarf::get_dwarf_info(), addr).unwrap();

	let sym = "unknown";// dwarf::parse_info_units(&dwarf::get_dwarf_info(), addr as u64).unwrap().unwrap_or("<unknown>");

	//let (sym, start_addr) = get_symbol_for_addr(addr).unwrap_or(("unknown", 0));

	Some((bound.name, bound.line as usize, sym, 0, usize::coerce(bound.address)))
}

pub fn get_symbol_for_addr(addr: usize) -> Option<(&'static str, usize)> {
	// SAFE: This should only ever be initialised once, and from an empty state
	let (symtab, addr_offset) = unsafe { (S_SYMS.symtab, S_SYMS.addr_offset) };
	let mut best = (!0, 0);
	println!(" ssearch {:x}", addr);
	for (i,s) in symtab.iter().enumerate()
	{
	println!(" sentry");
		if s.st_info & 0xF == 0x02
		{
			let base = s.st_value as usize + addr_offset;
			let len = s.st_size as usize;
			println!("- {} {:#x}+{:#x}", get_name(s.st_name as usize), base, len);
			if base != addr_offset && base <= addr {
				let ofs = addr - base;
				if len > 0 {
					if addr < base + len {
						return Some( (get_name(s.st_name as usize), ofs) );
					}
				}
				else {
					if ofs < best.0 {
						best = (ofs, i);
					}
				}
			}
		}
	}
	if best.1 != 0 {
		Some( (get_name(symtab[best.1].st_name as usize), best.0) )
	}
	else {
		None
	}
}


fn get_name(ofs: usize) -> &'static str {
	// SAFE: This should only ever be initialised once, and from an empty state
	let strtab = unsafe { S_SYMS.strtab };
	if ofs == 0 {
		""
	}
	else if ofs >= strtab.len() {
		println!("{:#x} >= {}", ofs, strtab.len());
		"#BADSTR#"
	}
	else {
		let start = &strtab[ofs..];
		let bytes = start.split(|&x| x == b'\0').next().unwrap();
		::core::str::from_utf8(bytes).unwrap_or("#UTF8#")
	}
}

/// Obtain the old RBP value and return address from a provided RBP value
pub fn backtrace(bp: u64) -> Option<(u64,u64)>
{
	if bp == 0 {
		return None;
	}
	if bp % 8 != 0 {
		return None;
	}
	/*if ! ::memory::buf_valid(bp as *const (), 16) {
		return None;
	}*/

	// [rbp] = oldrbp, [rbp+8] = IP
	// SAFE: Pointer access checked, any alias is benign
	unsafe
	{
		let ptr: *const [u64; 2] = ::core::mem::transmute(bp);
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
	let cur_bp: u64;
	// SAFE: Reads from bp
	unsafe{ asm!("mov %rbp, $0" : "=r" (cur_bp)); }
	print!("Backtrace: {:#x}\n{}", cur_bp, Backtrace(cur_bp as usize));
}
pub struct Backtrace(usize);
impl ::core::fmt::Display for Backtrace {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		let mut bp = u64::coerce(self.0);
		while let Option::Some((newbp, ip)) = backtrace(bp)
		{
			try!(write!(f, " {:#x}", ip));
			if let Some( (file, line, name, ofs, mofs) ) = get_symbol_info_for_addr(usize::coerce(ip) - 1) {
				try!(write!(f, "({}:{} {}+{:#x} M@{:#x})", file, line, Demangle(name), ofs + 1, mofs));
			}
			try!(write!(f, "\n"));
			bp = newbp;
		}
		Ok( () )
	}
}
