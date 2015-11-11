use std::mem;
use std::slice;
use std::io::{Read, Error, Cursor};

fn read_lebi128<R: Read>(r: &mut R) -> Result<i64, Error> {
    let mut result = 0u64;
    let mut shift = 0u64;
    let mut byte;
    loop {
        byte = try!(read::<u8, R>(r));
        result |= ((byte & 0x7F) as u64) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            break;
        }
    }

    if (shift < 8 * 8) && (byte & 0x40 != 0) {
        result |= -(1u64 << shift);
    }

    Ok(result as i64)
}

fn read_lebu128<R: Read>(r: &mut R) -> Result<u64, Error> {
    let mut result = 0u64;
    let mut shift = 0u64;
    loop {
        let byte = try!(read::<u8, R>(r));
        result |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
    }
    Ok(result)
}

fn read<T, R: Read>(r: &mut R) -> Result<T, Error> {
    unsafe {
        let mut v: T =  mem::uninitialized();
        let b = slice::from_raw_parts_mut(&mut v as *mut T as *mut u8, mem::size_of_val(&v));
        try!(r.read_exact(b));
        Ok(v)
    }
}

fn read_str<'s>(r: &mut Cursor<&'s [u8]>) -> Result<&'s str, Error> {
    let start = r.position() as usize;

    loop {
        let c: u8 = try!(read(r));
        if c == 0 {
            break
        }
    }

    unsafe {
        let bytes = &r.get_ref()[start..((r.position() - 1) as usize)];
        Ok(mem::transmute(bytes))
    }
}

fn read_abbrev_pair<R: Read>(r: &mut R) -> Result<Option<(u64, u64)>, Error> {
    let name = try!(read_lebu128(r));
    let form = try!(read_lebu128(r));
    //println!("  name: {}  form: {}", name, form);

    Ok(if name == 0 && form == 0 {
        None
    } else {
        Some((name, form))
    })
}

fn read_abbrev(r: &mut Cursor<&[u8]>, abbrev_code: u64) -> Result<(), Error> {
    loop {
        let p = r.position();
        let code = try!(read_lebu128(r));
        let tag = try!(read_lebu128(r));
        let children: u8 = try!(read(r));

        //println!("abbrev code @ {:#x}: {} tag: {} children {}", p, code, tag, children);

        if code == abbrev_code {
            break
        }

        while try!(read_abbrev_pair(r)).is_some() {}
    }

    Ok(())
}

pub struct DwarfInfo<'s> {
    pub info: &'s [u8],
    pub abbrev: &'s [u8],
    pub str: &'s [u8],
    pub line: &'s [u8],
}

pub fn get_dwarf_info() -> DwarfInfo<'static> {
	extern {
		static debug_line_start: u8;
		static debug_line_end: u8;

		static debug_abbrev_start: u8;
		static debug_abbrev_end: u8;

		static debug_str_start: u8;
		static debug_str_end: u8;

		static debug_info_start: u8;
		static debug_info_end: u8;
	}

    unsafe {
        let line = slice::from_raw_parts(&debug_line_start, offset(&debug_line_end) - offset(&debug_line_start));
        let abbrev = slice::from_raw_parts(&debug_abbrev_start, offset(&debug_abbrev_end) - offset(&debug_abbrev_start));
        let str = slice::from_raw_parts(&debug_str_start, offset(&debug_str_end) - offset(&debug_str_start));
        let info = slice::from_raw_parts(&debug_info_start, offset(&debug_info_end) - offset(&debug_info_start));

        DwarfInfo {
            info: info,
            abbrev: abbrev,
            str: str,
            line: line,
        }
   }
}

fn parse_info_unit<'s>(data: &mut Cursor<&'s [u8]>, dwarf: &DwarfInfo<'s>, target: u64) -> Result<Option<&'s str>, Error> {
    let mut adata = &mut Cursor::new(dwarf.abbrev);
    let mut sdata = &mut Cursor::new(dwarf.str);

    let unit_length: u32 = try!(read(data));
    let unit_end = data.position() + unit_length as u64;
    let version: u16 = try!(read(data));
    let abbrev_offset: u32 = try!(read(data));
    let ptr_size: u8 = try!(read(data));

    macro_rules! debug {
        ($($arg:tt)*) => (
            if false {
                print!($($arg)*);
            }
        );
    }

    debug!("unit_end: {:#x}", unit_end);
    debug!("version: {}", version);
    debug!("abbrev_offset: {}", abbrev_offset);
    debug!("ptr_size: {}", ptr_size);

    while data.position() < unit_end {
        let p = data.position();
        let abbrev_code = try!(read_lebu128(data));

        if abbrev_code == 0 {
            continue;
        }

        adata.set_position(abbrev_offset as u64);

        read_abbrev(adata, abbrev_code);

        let mut name = None;
        let mut low_pc = None;
        let mut high_pc = None;

        loop {
            let (attr, form) = match try!(read_abbrev_pair(adata)) {
                Some(p) => p,
                None => break
            };

            debug!("    <{:x}>   abbrev {} attr {} form: {} - ", data.position(), abbrev_code, attr, form);

            let mut str_val = None;
            let mut uint_val = None;

            match form {
                /* DW_FORM_addr */ 0x01 => {
                    let data: usize = try!(read(data));
                    debug!("DW_FORM_addr {:#x}", data);
                    uint_val = Some(data as u64);
                }
                /* DW_FORM_data2 */ 0x05 => {
                    let data: u16 = try!(read(data));
                    debug!("DW_FORM_data2 {:#x}", data);
                }
                /* DW_FORM_data4 */ 0x06 => {
                    let data: u32 = try!(read(data));
                    debug!("DW_FORM_data4 {:#x}", data);
                    uint_val = Some(data as u64);
                }
                /* DW_FORM_data1 */ 0x0b => {
                    let data: u8 = try!(read(data));
                    debug!("DW_FORM_data1 {:#x}", data);
                    uint_val = Some(data as u64);
                }
                /* DW_FORM_sdata */ 0x0d => {
                    let data = try!(read_lebi128(data));
                    debug!("DW_FORM_sdata {:#x}", data);
                }
                /* DW_FORM_strp */ 0x0e => {
                    let str_offset: u32 = try!(read(data));
                    sdata.set_position(str_offset as u64);
                    let str = try!(read_str(sdata));
                    debug!("DW_FORM_strp {:#x}  {}", str_offset, str);
                    str_val = Some(str);
                }
                /* DW_FORM_ref_addr */ 0x10 => {
                    let data: u32 = try!(read(data));
                    debug!("DW_FORM_ref_addr {:#x}", data);
                }
                /* DW_FORM_ref4 */ 0x13 => {
                    let data: u32 = try!(read(data));
                    debug!("DW_FORM_ref4 {:#x}", data);
                }
                /* DW_FORM_sec_offset */ 0x17 => {
                    let data: u32 = try!(read(data));
                    debug!("DW_FORM_sec_offset {:#x}", data);
                }
                /* DW_FORM_exprloc */ 0x18 => {
                    let len = try!(read_lebu128(data));
                    let p = data.position();
                    data.set_position(p + len);
                    debug!("DW_FORM_exprloc len {:#x}", len);
                }
                /* DW_FORM_flag_present */ 0x19 => {
                    debug!("DW_FORM_flag_present");
                }
                _ => {
                    panic!("Unknown form {:#x}", form);
                }
            }

            debug!("\n");

            match attr {
                /* DW_AT_low_pc */ 0x11 => {
                    low_pc = uint_val;
                    debug!("DW_AT_low_pc = {:#x}\n", uint_val.unwrap());

                }
                /* DW_AT_high_pc */ 0x12 => {
                    high_pc = uint_val;
                    debug!("DW_AT_high_pc = {:#x}\n", uint_val.unwrap());
                }
                /* DW_AT_linkage_name */ 0x6e => {
                    name = str_val;
                    debug!("DW_AT_linkage_name = {}\n", str_val.unwrap());
                }
                _ => {}
            }
        }

        match (name, low_pc, high_pc) {
            (Some(name), Some(low_pc), Some(high_pc)) => {
                debug!("COMPARING  {} {:#x}-{:#x} with {:#x}\n", name, low_pc, high_pc, target);

                if target >= low_pc && target < low_pc + high_pc {
                    return Ok(Some(name))
                };
            }
            _ => {}
        }
    }


    Ok(None)
}

pub fn parse_info_units<'s>(dwarf: &DwarfInfo<'s>, target: u64) -> Result<Option<&'s str>, Error> {
    let mut cursor = Cursor::new(dwarf.info);

    while (cursor.position() as usize) < dwarf.info.len() {
        let name = try!(parse_info_unit(&mut cursor, dwarf, target));
        if name.is_some() {
            return Ok(name);
        }
    }

    Ok(None)
}

pub struct Bound<'s> {
    target: u64,
    pub address: u64,
    pub name: &'s str,
    pub line: usize,
}

fn parse_line_unit<'s>(data: &mut Cursor<&'s [u8]>, bound: &mut Bound<'s>) -> Result<(), Error> {
    let offset = data.position();
    let unit_length: u32 = try!(read(data));
    let unit_end = data.position() + unit_length as u64;
    let version: u16 = try!(read(data));

    let header_length: u32 = try!(read(data));
    let post_header = data.position() + header_length as u64;
    let minimum_instruction_length: u8 = try!(read(data));
    let maximum_operations_per_instruction: u8 = try!(read(data));
    let line_base: i8 = try!(read(data));
    let line_range: u8 = try!(read(data));
    let opcode_base: u8 = try!(read(data));

    let mut print = false;

    macro_rules! debug {
        ($($arg:tt)*) => (
            if print {
                print!($($arg)*);
            }
        );
    }

    debug!("\nversion: {}\n", version);
    debug!("header_length: {}\n", header_length);
    debug!("post_header: {}\n", post_header);
    debug!("minimum_instruction_length: {}\n", minimum_instruction_length);
    debug!("maximum_operations_per_instruction: {}\n", maximum_operations_per_instruction);
    debug!("line_base: {}\n", line_base);
    debug!("line_range: {}\n", line_range);
    debug!("opcode_base: {}\n", opcode_base);
    debug!("offset: {:#x} next {:#x}\n", offset, unit_end);

    for _ in 0..(opcode_base - 1) {
        let e = try!(read_lebu128(data));
        debug!("opcode_base-e: has {} args\n", e);
    }

    loop {
        let dir = try!(read_str(data));

        if dir.is_empty() {
            break;
        }

        debug!("Directory: {}\n", dir);
    }

    let file_table_offset = data.position();
/*
    debug!(" The File Name Table (offset {:#x}):\n", data.position());
    debug!("  Entry	Dir	Time	Size	Name\n");

    let mut i = 1;

    loop {
        let file = try!(read_str(data));

        if file.is_empty() {
            break;
        }

        let dir_idx = try!(read_lebu128(data));
        let time = try!(read_lebu128(data));
        let file_size = try!(read_lebu128(data));

        debug!("  {}\t{}\t{}\t{}\t{}\n",  i, dir_idx, time, file_size, file);

        files.push(file);

        i += 1;
    }
*/
    data.set_position(post_header);

    let mut bound_file = None;

    let mut op_index;
    let mut line;
    let mut address;
    let mut file;
    let mut is_stmt;

    macro_rules! calc {
        ($adv:expr) => ({
            let op_advance = $adv;
            address += minimum_instruction_length as u64 * ((op_index as u64 + op_advance) / maximum_operations_per_instruction as u64);
            op_index = (op_index + op_advance) % maximum_operations_per_instruction as u64;
        });
    }

    macro_rules! reset {
        () => (
            op_index = 0;
            line = 1i64;
            address = 0u64;
            file = 1;
            is_stmt = 1;
        );
    }

    macro_rules! output {
        () => (
            if address < bound.target && address >= bound.address {
                bound.address = address;
                bound.line = line as u64 as usize;
                bound_file = Some(file);
            }
            /*if table {
                println!("      #### {}:{} {:#x}  ", files[file as usize], line, address);
            }*/
        );
    }

    debug!(" Line Number Statements:\n");

    reset!();

    while data.position() < unit_end as u64 {
        let mut opcode: u8 = try!(read(data));

        debug!("  [0x{:08x}]  ", data.position() - 1);

        if opcode < opcode_base {
            match opcode {
                /* extended opcode */ 0 => {
                    let len = try!(read_lebu128(data));
                    let ecode: u8 = try!(read(data));

                    match ecode {
                        /* DW_LNE_end_sequence */ 0x1 => {
                            debug!("Extended opcode 1: End of Sequence\n\n");
                            reset!();
                        }
                        /* DW_LNE_set_address */ 0x2 => {
                            address = try!(read(data));
                            debug!("Extended opcode 2: set Address to {:#x}\n", address);
                        }
                        /* DW_LNE_define_file */ 0x3 => {
                            panic!();
                        }
                        _ => {
                            panic!("Unknown extended opcode {:#x}", ecode)
                        }
                    }

                }
                /* DW_LNS_copy */ 0x1 => {
                    output!();
                    debug!("Copy\n");
                }
                /* DW_LNS_advance_pc */ 0x2 => {
                    let old_address = address;

                    let op_advance = try!(read_lebu128(data));

                    calc!(op_advance);

                    debug!("Advance PC by {} to {:#x}\n",  address - old_address, address);
                }
                /* DW_LNS_advance_line */ 0x3 => {
                    let old_line = line;

                    let advance = try!(read_lebi128(data));
                    line += advance;

                    debug!("Advance Line by {} to {}\n", advance, line);
                }
                /* DW_LNS_set_file */ 0x4 => {
                    file = try!(read_lebu128(data));
                    debug!("Set File Name to entry {} in the File Name Table\n", file);
                }
                /* DW_LNS_set_column */ 0x5 => {
                    panic!();
                }
                /* DW_LNS_negate_stmt */ 0x6 => {
                    is_stmt = !is_stmt & 1;
                    debug!("Set is_stmt to {}\n", is_stmt);
                }
                /* DW_LNS_set_basic_block */ 0x7 => {
                    panic!();
                }
                /* DW_LNS_const_add_pc */ 0x8 => {
                    let old_address = address;

                    let op_advance = (255 - opcode_base) as u64 / line_range as u64;
                    calc!(op_advance);

                    debug!("Advance PC by constant {} to {:#x}\n",  address - old_address, address);
                }
                /* DW_LNS_fixed_advance_pc */ 0x9 => {
                    panic!();
                }
                /* DW_LNS_set_prologue_end */ 0xA => {
                    debug!("Set prologue_end to true\n");
                }
                /* DW_LNS_set_epilogue_begin */ 0xB => {
                    panic!();
                }
                /* DW_LNS_set_isa */ 0xC => {
                    panic!();
                }
                _ => {
                    panic!("Unknown opcode {:#x}", opcode)
                }
            }
        } else {
            opcode -= opcode_base;//opcode.wrapping_sub(opcode_base);

            let op_advance = opcode as u64 / line_range as u64;

            let old_address = address;

            calc!(op_advance);

            let old_line = line;

            line += line_base as i64 + (opcode % line_range) as i64;

            output!();

            debug!("Special opcode {}: advance Address by {} to {:#x} and Line by {} to {}\n", opcode, address - old_address, address, line - old_line, line);
        }
    }

    assert!(data.position() == unit_end);

    if let Some(file_index) = bound_file {
        data.set_position(file_table_offset);

        let mut i = 1;

        loop {
            let file = try!(read_str(data));

            if file.is_empty() {
                break;
            }

            let dir_idx = try!(read_lebu128(data));
            let time = try!(read_lebu128(data));
            let file_size = try!(read_lebu128(data));

            debug!("  {}\t{}\t{}\t{}\t{}\n",  i, dir_idx, time, file_size, file);

            if i == file_index {
                bound.name = file;
                break;
            }

            i += 1;
        }
    }

    data.set_position(unit_end);

    Ok(())
}

pub fn parse_line_units<'s>(dwarf: &DwarfInfo<'s>, target: usize) -> Result<Bound<'s>, Error> {
    let mut bound = Bound {
        target: target as u64,
        address: 0,
        line: 1,
        name: "<unknown>",
    };

    let mut cursor = Cursor::new(dwarf.line);

    while (cursor.position() as usize) < dwarf.line.len() {
        try!(parse_line_unit(&mut cursor, &mut bound));
    }

    Ok(bound)
}
