
pub const HEADER_FLAG_PAGE_ALIGN: u32 = 1 << 0;
pub const HEADER_FLAG_MEMORY_INFO: u32 = 1 << 1;

pub const MAGIC: u32 = 0x2BADB002;

pub const HEADER_MAGIC: u32 = 0x1BADB002;

#[repr(packed)]
pub struct Header {
    pub magic: u32,
    pub flags: u32,
    pub checksum: u32
}