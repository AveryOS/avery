use std;
use std::iter::Iterator;
use params;
use util::FixVec;
use memory::{Addr};
use arch::symbols;

mod multiboot;

#[no_mangle]
pub extern "C" fn boot_entry(info: &multiboot::Info) {
	init(info);
}

#[inline(never)]
pub fn init(info: &multiboot::Info) {
	use elfloader::{self, Image, elf};

	extern {
		static low_end: void;
		static kernel_start: void;
		static rodata_start: void;
		static data_start: void;
		static kernel_end: void;
	}

	fn setup_segment(params: &mut params::Info, kind: params::SegmentKind, virtual_start: &'static void, virtual_end: &'static void)
	{
		let base = offset(virtual_start) - offset(&kernel_start) + offset(&low_end);

		params.segments.push(params::Segment {
			kind: kind,
			base: base as Addr,
			end: (base + (offset(virtual_end) - offset(virtual_start))) as Addr,
			virtual_base: offset(virtual_start),
			found: false,
			name: unsafe { std::mem::zeroed() }
		});
	}

    ::arch::console::cls();

	if info.flags & multiboot::FLAG_MMAP == 0 {
		panic!("Memory map not passed by Multiboot loader");
	}

	if info.flags & multiboot::FLAG_ELF == 0 {
		panic!("ELF symbols not passed by Multiboot loader");
	}

	let first_2m = unsafe {
		std::slice::from_raw_parts(0 as *const u8, 0x200000)
	};

	let bin = Image::new_sections(first_2m, u64::from(info.addr), u16::coerce(info.num), u16::coerce(info.size), u16::coerce(info.shndx)).unwrap();

	let mut params = params::Info {
		ranges: FixVec::new(),
		segments: FixVec::new(),
		symbols: params::Symbols {
			base: info.addr as u64,
			count: u16::coerce(info.num),
			strtab: u16::coerce(info.shndx),
		},
	};

	params.segments.push(params::Segment {
		kind: params::SegmentKind::Symbols,
		base: Addr::from(info.addr),
		end: Addr::from(info.addr + info.num * info.size),
		virtual_base: 0,
		found: false,
		name: unsafe { std::mem::zeroed() }
	});

	// Place addr in offset so elfloader will use the correct data
	for section in bin.sections.iter() {
		unsafe {
			*(&section.offset as *const u64 as *mut u64) = section.addr;
		}
	}

	for section in bin.sections.iter() {
		if section.shtype != elf::SHT_SYMTAB &&
		   section.shtype != elf::SHT_PROGBITS &&
		   section.shtype != elf::SHT_STRTAB {
			continue;
		}

		let name = bin.section_name(section).unwrap();

		if section.shtype == elf::SHT_PROGBITS &&
			!name.starts_with(".debug_") {
			continue;
		}

		params.segments.push(params::Segment {
			kind: params::SegmentKind::Symbols,
			base: section.addr as Addr,
			end: (section.addr + section.size) as Addr,
			virtual_base: 0,
			found: false,
			name: unsafe { std::mem::zeroed() }
		});
	}

	setup_segment(&mut params, params::SegmentKind::Code, &kernel_start, &rodata_start);
	setup_segment(&mut params, params::SegmentKind::ReadOnlyData, &rodata_start, &data_start);
	setup_segment(&mut params, params::SegmentKind::Data, &data_start, &kernel_end);

	for i in 0..info.mods_count {
		let module = unsafe { &*(info.mods_addr as *const multiboot::Module).offset(isize::coerce(i)) };

		let segment = params::Segment {
			kind: params::SegmentKind::Module,
			base: module.start as Addr,
			end: module.end as Addr,
			virtual_base: 0,
			found: false,
			name: unsafe { std::mem::zeroed() }
		};

		params.segments.push(segment);
/*
		*/
		/*
		size_t name_size = sizeof(segment.name) - 1;

		strncpy(segment.name, (char *)mod->name, name_size);

		segment.name[name_size] = 0;*/
	}

	let mmap_end = (info.mmap_addr + info.mmap_length) as usize;

	let mut mmap = unsafe { &*(info.mmap_addr as *const multiboot::MemoryMap) };

	while offset(mmap) < mmap_end {
		if mmap.kind == 1 {
			params.ranges.push(params::Range {
				kind: params::MemoryKind::Usable,
				base: mmap.base as Addr,
				end: (mmap.base + mmap.size) as Addr,
				next: std::ptr::null_mut()
			});
		}
		mmap = unsafe { &*((offset(mmap) + mmap.struct_size as usize + 4) as *const multiboot::MemoryMap) };
	}

	::init(&mut params);
}
