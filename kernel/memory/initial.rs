use arch;
use memory::physical;
use memory::Addr;
use params;
use params::Range;
use util::FixVec;

pub struct State<'a> {
	pub info: &'a mut params::Info,
	pub holes: usize,
	pub overhead: usize,
	pub list: *mut Range,
	pub entry: *mut Range // The entry used to store allocator data
}

unsafe fn load_memory_map(info: &mut params::Info) -> *mut Range {
	let mut list = null_mut();

	for entry in info.ranges.iter_mut() {
		if entry.kind == params::MemoryKind::Usable {
			if entry.base < arch::PHYS_PAGE_SIZE * 2 { // Ignore the first two pages
				entry.base = arch::PHYS_PAGE_SIZE * 2;
			}

			entry.next = list;
			list = entry as *mut Range;
		}
	}

	list
}

unsafe fn punch_holes(st: &mut State) {
	let mut entry_ = st.list;
	let mut prev = entry_;

	'label: while entry_ != null_mut() {
		let entry = &mut *entry_;

		for hole in st.info.segments.iter_mut() {
			if hole.found {
				continue;
			}

			if hole.base >= entry.base && hole.base < entry.end {
				// The hole starts in this entry.

				assert!(hole.end <= entry.end); // Make sure it ends here too!

				hole.found = true;

				if hole.base == entry.base && hole.end == entry.end {
					// The entry and hole match perfectly. Remove the entry from the list.

					if prev == st.list {
						st.list = entry.next;
						prev = st.list;
					} else {
						(*prev).next = entry.next;
					}

					entry_ = entry.next;
					continue 'label;
				} else if hole.base == entry.base {
					// The entry's and hole's bases match perfectly. Resize the entry.

					entry.base = hole.end;
				} else if hole.end == entry.end {
					// The entry's and hole's ends match perfectly. Resize the entry.

					entry.end = hole.base;
				} else {
					// There is space before and after the hole. Allocate a new hole.

					let entry_end = entry.end;

					entry.end = hole.base;

					st.info.ranges.push(params::Range {
						kind: params::MemoryKind::Usable,
						base: hole.end,
						end: entry_end,
						next: entry.next
					});

					let len = st.info.ranges.len() - 1;
					entry.next = &mut st.info.ranges[len] as *mut Range;
				}
			} else {
				assert!(hole.end <= entry.base || hole.end > entry.end); // The hole ends, but doesn't start in this entry.
			}
		}

		prev = entry_;

		entry_ = entry.next;
	}

	for hole in st.info.segments.iter() {
		if !hole.found {
			panic!("Unable to find room for hole ({:x}) - ({:x})", hole.base, hole.end);
		}
	}
}

unsafe fn align_holes(st: &mut State) {
	let mut entry_ = st.list;
	let mut prev = entry_;

	while entry_ != null_mut() {
		let entry = &mut *entry_;

		entry.base = align_up(entry.base, arch::PHYS_PAGE_SIZE);
		entry.end = align_down(entry.end, arch::PHYS_PAGE_SIZE);

		if entry.end > entry.base {
			prev = entry_; // Go to the next entry
		} else {
			// No usable memory in this entry. Remove it from the list.

			if prev == st.list {
				st.list = entry.next;
				prev = st.list;
			} else {
				(*prev).next = entry.next;
			}
		}

		entry_ = entry.next;
	}
}

unsafe fn find_biggest_entry(mut entry_: *mut Range) -> Option<*mut Range> {
	let mut result: Option<*mut Range> = None;

	while entry_ != null_mut() {
		let entry = &mut *entry_;

		match result {
			Some(r) => {
				if entry.end - entry.base > (*r).end - (*r).base {
					result = Some(entry_);
				}
			}
			None => result = Some(entry_)
		}

		entry_ = entry.next;
	}

	result
}

pub unsafe fn initialize_physical(info: &mut params::Info) -> State {
	let mut st = State {
		info: info,
		holes: 0,
		overhead: 0,
		list: null_mut(),
		entry: null_mut()
	};

	st.list = load_memory_map(st.info);

	if st.list == null_mut() {
		panic!("No usable memory found!");
	}

	punch_holes(&mut st);

	if st.list == null_mut() {
		panic!("No usable memory found after reserving holes!");
	}

	align_holes(&mut st);

	if st.list == null_mut() {
		panic!("No usable memory found after removing non-page aligned holes!");
	}

	st.entry = find_biggest_entry(st.list).unwrap();

	let mut memory_in_pages = 0;

	let mut entry_ = st.list;

	while entry_ != null_mut() {
		let entry = &mut *entry_;

		println!("Free physical memory {:#x} - {:#x}", entry.base, entry.end);

		memory_in_pages += (entry.end - entry.base) / arch::PHYS_PAGE_SIZE;
		st.overhead += size_of::<physical::Hole>() + size_of::<usize>() * usize::coerce(div_up(entry.end - entry.base, physical::BYTE_MAP_SIZE));

		st.holes += 1;

		entry_ = entry.next;
	}

	println!("Available memory: {} MiB", memory_in_pages * arch::PHYS_PAGE_SIZE / 0x100000);

	assert!(st.overhead as Addr <= (*st.entry).end - (*st.entry).base); // Memory allocation overhead is larger than the biggest memory block
	assert!(st.overhead <= arch::memory::MAX_OVERHEAD); // Memory map doesn't fit in 2 MB.

	st
}
