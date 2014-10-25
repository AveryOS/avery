#![allow(dead_code)]

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

pub const FLAG_MEM: u32 = 0x001;
pub const FLAG_DEVICE: u32 = 0x002;
pub const FLAG_CMDLINE: u32 = 0x004;
pub const FLAG_MODS: u32 = 0x008;
pub const FLAG_AOUT: u32 = 0x010;
pub const FLAG_ELF: u32 = 0x020;
pub const FLAG_MMAP: u32 = 0x040;
pub const FLAG_CONFIG: u32 = 0x080;
pub const FLAG_LOADER: u32 = 0x100;
pub const FLAG_APM: u32 = 0x200;
pub const FLAG_VBE: u32 = 0x400;

#[repr(packed)]
pub struct MemoryMap {
	pub struct_size: u32,
	pub base: u64,
	pub size: u64,
	pub kind: u32
}

#[repr(packed)]
pub struct Module {
	pub start: u32,
	pub end: u32,
	pub name: u32,
	pub reserved: u32,
}

#[repr(packed)]
pub struct Info {
	pub flags: u32,
	pub mem_lower: u32,
	pub mem_upper: u32,
	pub boot_device: u32,
	pub cmdline: u32,
	pub mods_count: u32,
	pub mods_addr: u32,
	pub num: u32,
	pub size: u32,
	pub addr: u32,
	pub shndx: u32,
	pub mmap_length: u32,
	pub mmap_addr: u32,
	pub drives_length: u32,
	pub drives_addr: u32,
	pub config_table: u32,
	pub boot_loader_name: u32,
	pub apm_table: u32,
	pub vbe_control_info: u32,
	pub vbe_mode_info: u32,
	pub vbe_mode: u32,
	pub vbe_interface_seg: u32,
	pub vbe_interface_off: u32,
	pub vbe_interface_len: u32,
}
