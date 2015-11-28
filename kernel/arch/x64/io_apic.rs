use arch;
use util::FixVec;
use memory;
use memory::{PhysicalPage, Addr};

pub struct IOAPIC {
	id: u8,
	irq_base: u32,
	irq_count: u32,
	registers: usize,
}

fix_array_struct!(IOAPICVec, MAX_IO_APICS);

const MAX_IO_APICS: usize = 32;

static mut IOS: Option<IOAPICVec<IOAPIC>> = None;

const REG_ID: u32 = 0;
const REG_VERSION: u32 = 1;
const REG_IRQ_START: u32 = 0x10;

const MASK_BIT: u32 = 1 << 16;
const LEVEL_SENSITIVE_BIT: u32 = 1 << 15;
const ACTIVE_LOW_BIT: u32 = 1 << 13;

pub unsafe fn initialize(ios: IOAPICVec<IOAPIC>) {
    IOS = Some(ios);
}

impl IOAPIC {
    pub unsafe fn new(base: u32, id: u8, registers: Addr) -> IOAPIC {
        let mut io = IOAPIC {
            id: id,
            irq_base: base,
            irq_count: 0,
            registers: memory::map_physical(PhysicalPage::new(registers), 1, arch::memory::RW_DATA_FLAGS | arch::memory::NO_CACHE_FLAGS).1.ptr(),
        };

    	let id_from_reg = (io.get_reg(REG_ID) >> 24) & 0xF;

    	if id_from_reg as u8 != id {
    		println!("I/O APIC register id differs from ACPI id: {} vs. ACPI: {}", id_from_reg, id);
        }

    	let version = io.get_reg(REG_VERSION);

    	io.irq_count = ((version >> 16) & 0xFF) + 1;

    	println!("I/O APIC {} base: {} IRQ count: {} vers {} regs {:#x}", io.id, base, io.irq_count, version, registers);

    	// Mask all IRQs

    	for i in 0..io.irq_count {
            let reg = REG_IRQ_START + i * 2;
    		io.reg(reg, io.get_reg(reg) | MASK_BIT);
        }

    	io
    }

	unsafe fn get_reg(&self, reg: u32) -> u32 {
		volatile_store(self.registers as *mut u32, reg);
		volatile_load((self.registers + 16) as *mut u32)
	}

	unsafe fn reg(&self, reg: u32, val: u32) {
		volatile_store(self.registers as *mut u32, reg);
		volatile_store((self.registers + 16) as *mut u32, val);
	}

    unsafe fn route(&self, irq: usize, vector: u8, target: u8, edge_triggered: bool, active_low: bool) {
    	assert!(irq < self.irq_count as usize, "IRQ index out of bounds");

        println!("Routing IRQ {} to target {}", irq, target);

    	let reg_start = REG_IRQ_START + (irq as u32) * 2;

		// Mask the Interrupt before changing it
    	self.reg(reg_start, self.get_reg(reg_start) | MASK_BIT);
		
    	self.reg(reg_start + 1, (target as u32) << 24);
        let mut val = vector as u32;
        if !edge_triggered {
            val |= LEVEL_SENSITIVE_BIT;
        }
        if active_low {
            val |= ACTIVE_LOW_BIT;
        }
    	self.reg(reg_start, val);
    }
}

pub struct IRQ {
	apic: Option<&'static IOAPIC>,
	pub index: usize,
	edge_triggered: bool,
	active_low: bool,
}

impl IRQ {
    pub fn new(index: usize, edge_triggered: bool, active_low: bool) -> IRQ {
        IRQ {
            index: index,
            apic: None,
            edge_triggered: edge_triggered,
            active_low: active_low,
        }
    }

    unsafe fn setup(&mut self) {
    	if self.apic.is_some() {
    		return;
        }

        let ios = IOS.as_ref().unwrap();

        for io in ios.iter() {
    		if (self.index as u32) >= io.irq_base && (self.index as u32) < io.irq_base + io.irq_count {
    			self.apic = Some(io);
    			self.index -= io.irq_base as usize;
    			return;
    		}
        }

    	panic!("Unable to find interrupt from id");
    }

    pub unsafe fn route(&mut self, vector: u8, target: u8) {
        self.setup();
        self.apic.unwrap().route(self.index, vector, target, self.edge_triggered, self.active_low);
    }
}
