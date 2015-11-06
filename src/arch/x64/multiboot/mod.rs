use std;
use params;
use util::FixVec;

mod multiboot;

#[no_mangle]
pub extern "C" fn boot_entry(info: &multiboot::Info) {
	init(info);
	::kernel();
}

#[inline(never)]
pub fn init(info: &multiboot::Info) {
    ::arch::console::cls();

	if info.flags & multiboot::FLAG_MMAP == 0 {
		panic!("Memory map not passed by Multiboot loader");
	}

	let mut params = params::Info {
		ranges: FixVec::new(),
		segments: FixVec::new()

	};

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
			base: base as uphys,
			end: (base + offset(virtual_end) - offset(virtual_start)) as uphys,
			virtual_base: offset(virtual_start),
			found: false,
			name: unsafe { std::mem::zeroed() }
		});
	}

	setup_segment(&mut params, params::SegmentKind::Code, &kernel_start, &rodata_start);
	setup_segment(&mut params, params::SegmentKind::ReadOnlyData, &rodata_start, &data_start);
	setup_segment(&mut params, params::SegmentKind::Data, &data_start, &kernel_end);

	for i in 0..info.mods_count {
		let module = unsafe { &*(info.mods_addr as *const multiboot::Module).offset(i as isize) };

		let segment = params::Segment {
			kind: params::SegmentKind::Module,
			base: module.start as uphys,
			end: module.end as uphys,
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
				base: mmap.base as uphys,
				end: (mmap.base + mmap.size) as uphys,
				next: std::ptr::null_mut()
			});
		}
		mmap = unsafe { &*((offset(mmap) + mmap.struct_size as usize + 4) as *const multiboot::MemoryMap) };
	}

	::init(&mut params);
}
