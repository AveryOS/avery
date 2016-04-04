use util::FixVec;
use memory::Addr;

#[derive(Eq, PartialEq)]
pub enum MemoryKind {
	Usable,
	ACPI
}

#[repr(C)]
pub struct Range {
	pub kind: MemoryKind,
	pub base: Addr,
	pub end: Addr,
	pub next: *mut Range
}

#[derive(Eq, PartialEq, Debug)]
pub enum SegmentKind {
	Code,
	ReadOnlyData,
	Data,
	Module,
	Symbols,
}

#[repr(C)]
pub struct Segment {
	pub kind: SegmentKind,
	pub base: Addr,
	pub end: Addr,
	pub virtual_base: usize,
	pub found: bool,
	pub name: [u8; 0x100]
}

fix_array_struct!(MemoryRangeVec, 0x100);
fix_array_struct!(SegmentVec, 0x20);

#[repr(C)]
pub struct Symbols {
	pub base: u64,
	pub count: u16,
	pub strtab: u16,
}

#[repr(C)]
pub struct Info {
	pub ranges: MemoryRangeVec<Range>,
	pub segments: SegmentVec<Segment>,
	pub symbols: Symbols,
}
