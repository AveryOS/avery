use util::FixVec;

#[derive(Eq, PartialEq)]
pub enum MemoryKind {
	Usable,
	ACPI
}

#[repr(C)]
pub struct Range {
	pub kind: MemoryKind,
	pub base: uphys,
	pub end: uphys,
	pub next: *mut Range
}

#[derive(Eq, PartialEq)]
pub enum SegmentKind {
	Code,
	ReadOnlyData,
	Data,
	Module
}

#[repr(C)]
pub struct Segment {
	pub kind: SegmentKind,
	pub base: uphys,
	pub end: uphys,
	pub virtual_base: usize,
	pub found: bool,
	pub name: [u8; 0x100]
}

fix_array_struct!(MemoryRangeVec, 0x100);
fix_array_struct!(SegmentVec, 0x10);

#[repr(C)]
pub struct Info {
	pub ranges: MemoryRangeVec<Range>,
	pub segments: SegmentVec<Segment>
}
