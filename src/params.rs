use util::FixVec;

#[deriving(Eq, PartialEq)]
pub enum MemoryKind {
	MemoryNone,
	MemoryUsable,
	MemoryACPI
}

#[repr(C)]
pub struct Range {
	pub kind: MemoryKind,
	pub base: uphys,
	pub end: uphys,
	pub next: *mut Range
}

#[deriving(Eq, PartialEq)]
pub enum SegmentKind {
	SegmentCode,
	SegmentReadOnlyData,
	SegmentData,
	SegmentModule
}

#[repr(C)]
pub struct Segment {
	pub kind: SegmentKind,
	pub base: uphys,
	pub end: uphys,
	pub virtual_base: uptr,
	pub found: bool,
	pub name: [u8, ..0x100]
}

fix_array_struct!(MemoryRangeVec, 0x100)
fix_array_struct!(SegmentVec, 0x10)

#[repr(C)]
pub struct Info {
	pub ranges: MemoryRangeVec<Range>,
	pub segments: SegmentVec<Segment>
}
