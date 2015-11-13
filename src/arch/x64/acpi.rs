use util::FixVec;
use arch::memory::R_DATA_FLAGS;
use arch::{io_apic, IRQ, apic};
use cpu::CPUVec;
use std;
use std::slice;
use memory::{Addr, PhysicalView, offset};

pub struct CPUInfo {
	pub acpi_id: usize,
	pub apic_id: usize,
}

#[derive(Copy, Clone)]
#[repr(packed)]
struct RSDP {
	signature: [u8; 8],
	checksum: u8,
	oem: [u8; 6],
	revision: u8,
	address: u32,
}

#[repr(packed)]
struct SDT {
	signature: [u8; 4],
	length: u32,
	revision: u8,
	checksum: u8,
	oem_id: [u8; 6],
	oem_table_id: [u8; 8],
	oem_revision: u32,
	creator_id: u32,
	creator_revision: u32,
}

const KIND_PROCESSOR_LOCAL_APIC: u8 = 0;
const KIND_IOAPIC: u8 = 1;
const KIND_INTERRUPT_SOURCE_OVERRIDE: u8 = 2;
const KIND_NMI_SOURCE: u8 = 3;
const KIND_LOCAL_APIC_NMI: u8 = 4;
const KIND_LOCAL_APIC_ADDRESS_OVERRIDE: u8 = 5;

const FLAG_ENABLED: u32 = 1;

#[repr(packed)]
struct MADTEntry {
	kind: u8,
	length: u8,
}

#[repr(packed)]
struct ProcessorLocalAPIC {
	entry: MADTEntry,

	processor_id: u8,
	apic_id: u8,
	flags: u32,
}

#[repr(packed)]
struct IOAPIC {
	entry: MADTEntry,
	id: u8,
	reserved: u8,
	address: u32,
	global_int_start: u32,
}

#[repr(packed)]
struct LocalAPICAddressOverride {
	entry: MADTEntry,
	reserved: u16,
	apic_address: u64,
}

#[repr(packed)]
struct InterruptSourceOverride {
	entry: MADTEntry,
	bus: u8,
	source: u8,
	global_int: u32,
	/*
	unsigned int polarity : 2;
	unsigned int trigger_mode : 2;
	unsigned int reserved : 12;

	*/
	misc: u16,
}

impl InterruptSourceOverride {
	fn polarity(&self) -> u16 {
		self.misc >> 14
	}
	fn trigger_mode(&self) -> u16 {
		(self.misc >> 12) & 3
	}
}

#[repr(packed)]
struct MADT {
	sdt: SDT,

	local_interrupt_controller: u32,
	flags: u32,

}

#[repr(packed)]
struct RSDT {
	sdt: SDT,
	tables: [u32; 0], // Variable length
}

const RSDP_SIGNATURE_MAGIC: &'static str = "RSD PTR ";
const RSDT_SIGNATURE_MAGIC: &'static str = "RSDT";
const MADT_SIGNATURE_MAGIC: &'static str = "APIC";

const BIOS_START: usize = 0xE0000;
const BIOS_END: usize = 0x100000;

fn checksum(mem: &[u8]) -> u8 {
	mem.iter().fold(0, |acc, &b| acc + b)
}

unsafe fn assert_valid(table: *const SDT) {
	let view = slice::from_raw_parts(table as *const u8, (*table).length as usize);
	assert!(checksum(view) == 0, "Invalid checksum");
}

unsafe fn search_area(start: Addr, size: usize) -> Option<RSDP> {
	let mut scoped = PhysicalView::new();
	let view = scoped.map(start, size, R_DATA_FLAGS);

	let mut i = 0;

	while i < size {
		let ptr = &*(&view[i] as *const u8 as *const RSDP);

		if ptr.signature == RSDP_SIGNATURE_MAGIC.as_bytes() {
			let view = &view[i..(i + std::mem::size_of::<RSDP>())];

			if checksum(view) == 0 {
				return Some(*ptr);
			}
		}

		i += 16;
	}

	None
}

unsafe fn search() -> RSDP {
	if let Some(rsdp) = search_area(BIOS_START as Addr, BIOS_END - BIOS_START) {
		return rsdp;
	}

	let mut ebda_map = PhysicalView::new();
	let ebda = (*ebda_map.map_object::<u16>(0x40E, R_DATA_FLAGS) as Addr) << 4;

	if let Some(rsdp) = search_area(align_up(ebda, 16), 0x400) {
		return rsdp;
	}

	panic!("Didn't find the ACPI RSDP structure");
}

unsafe fn load_table(view: &mut PhysicalView, address: Addr) -> *const SDT {
	let mut sdt_map = PhysicalView::new();
	let sdt = sdt_map.map_object::<SDT>(address, R_DATA_FLAGS);
	let sdt = view.map(address, sdt.length as usize, R_DATA_FLAGS).as_ptr() as *const SDT;
	assert_valid(sdt);
	sdt
}

pub struct Setup {
	pub cpus: CPUVec<CPUInfo>,
	pub pit_irq: IRQ,
	pub ios: io_apic::IOAPICVec<io_apic::IOAPIC>,
	pub apic_address: Option<Addr>,
}

unsafe fn parse_madt(madt: *const MADT, setup: &mut Setup) {
	//APIC::set_registers(madt->local_interrupt_controller);

	let mut entry = offset(madt, 1) as *const MADTEntry;
	let end = (madt as usize + (*madt).sdt.length as usize) as *const MADTEntry;

	while entry < end {

		match (*entry).kind {
			KIND_PROCESSOR_LOCAL_APIC => {
				let processor = &*(entry as *const ProcessorLocalAPIC);

				if processor.flags & FLAG_ENABLED != 0 {
					setup.cpus.push(CPUInfo {
						acpi_id: processor.processor_id as usize,
						apic_id: processor.apic_id as usize,
					});
				}
			}
			KIND_IOAPIC => {
				let io = &*(entry as *const IOAPIC);
				setup.ios.push(io_apic::IOAPIC::new(io.global_int_start, io.id, io.address as Addr));
			}
			KIND_INTERRUPT_SOURCE_OVERRIDE => {
				let iso = &*(entry as *const InterruptSourceOverride);

				if iso.bus == 0 && iso.source as usize == setup.pit_irq.index {
					assert!(iso.polarity() != 2, "Unknown polarity");
					assert!(iso.trigger_mode() != 2, "Unknown trigger mode");

					setup.pit_irq = IRQ::new(iso.global_int as usize, iso.trigger_mode() != 3, iso.polarity() == 3);
				}

				println!("Interrupt source override - bus: {} irq: {} int: {}", iso.bus, iso.source, iso.global_int);
			}
			KIND_LOCAL_APIC_ADDRESS_OVERRIDE => {
				let laao = &*(entry as *const LocalAPICAddressOverride);
				println!("Local APIC Address Override - address: {:#x}", laao.apic_address);
				setup.apic_address = Some(laao.apic_address as Addr);
			}
			_ => {}
		}

		entry = (entry as usize + (*entry).length as usize) as *const MADTEntry;
	}

}

pub unsafe fn initialize(pit_irq: IRQ) -> Setup {
	/*if(has_table)
	{
		Memory::ScopedBlock block;

		rsdp = *block.map_object<RSDP>(acpi_table);

		assert(rsdp.signature == RSDP::signature_magic, "Invalid ACPI RSDP signature");
		assert(checksum((uint8_t *)&rsdp, (uint8_t *)(&rsdp + 1)) == 0, "Invalid ACPI RSDP checksum");
	}
	else*/

	let mut setup = Setup {
		cpus: CPUVec::new(),
		pit_irq: pit_irq,
		ios: io_apic::IOAPICVec::new(),
		apic_address: None,
	};

	let rsdp = search();

	let mut rsdt_block = PhysicalView::new();

	let rsdt = &*(load_table(&mut rsdt_block, rsdp.address as Addr) as *const RSDT);

	assert!(rsdt.sdt.signature == RSDT_SIGNATURE_MAGIC.as_bytes(), "Invalid ACPI RSDT table magic");

	let tables = (rsdt.sdt.length as usize - size_of::<SDT>()) / size_of::<u32>();

	for i in 0..tables {
		let mut table_block = PhysicalView::new();

		let table = &*load_table(&mut table_block, *rsdt.tables.get_unchecked(i) as Addr);

		println!("Found ACPI Table: {}", std::str::from_utf8(&table.signature).unwrap());

		if table.signature == MADT_SIGNATURE_MAGIC.as_bytes() {
			parse_madt(table as *const SDT as *const MADT, &mut setup);
		}

	}

	setup
}
