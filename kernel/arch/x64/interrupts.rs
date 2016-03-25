use arch;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;

unsafe fn setup_pics() {
	use arch::outb;

	let master_command = 0x20;
	let master_data = 0x21;
	let slave_command = 0xA0;
	let slave_data = 0xA1;

	let pic_init = 0x11;

	let pic_mask_all = 0xFF;

	// Remap the PICs IRQ tables

	outb(master_command, pic_init);
	outb(master_data, 0xF8);
	outb(master_data, 0x04);
	outb(master_data, 0x01);
	outb(master_data, 0x0);

	outb(slave_command, pic_init);
	outb(slave_data, 0xF8);
	outb(slave_data, 0x02);
	outb(slave_data, 0x01);
	outb(slave_data, 0x0);

	// Disable the PICs

	outb(master_data, pic_mask_all);
	outb(slave_data, pic_mask_all);
}

pub unsafe fn enable() {
	asm!("sti");
}

pub unsafe fn disable() {
	asm!("cli");
}

#[allow(dead_code)]
#[repr(packed)]
pub struct Info {
	ds: u16,
	padding: [u16; 3],
	registers: arch::GeneralRegisters,
	ss: u64,
}

pub type Handler = extern fn (info: &Info, index: u8, error_code: usize);

const HANDLER_COUNT: usize = 256; // Same as in interrupts.s

extern {
	#[link_name = "interrupt_handlers"]
	pub static mut HANDLERS: [AtomicUsize; HANDLER_COUNT];

	#[link_name = "isr_stubs"]
	static ISR_STUBS: [unsafe extern fn(); HANDLER_COUNT - 1];

	fn spurious_irq();
}

extern fn nmi_handler(_: &Info, _: u8, _: usize) {
	panic!("Non-maskable interrupt");
}

extern fn page_fault_handler(info: &Info, _: u8, error_code: usize)
{
	let cr2: u64;

	unsafe {
	    asm! {
	    	[%rax => cr2]

	    	mov rax, cr2
	    }
	}

	;

	let access = if (error_code & (1 << 4)) != 0 {
		"executing"
	} else if (error_code & (1 << 1)) == 0 {
		"reading"
	} else {
		"writing"
	};

	let reason = if (error_code & 1) == 0 {
		"Page not present"
	} else if (error_code & (1 << 3)) != 0 {
		"Reserved bit set"
	} else {
		"Unknown"
	};

    panic!("Page fault {} {:#x} ({})\n\nerrnr: {:#x} cpu: {} rsi: {:x}  rsp: {:x} rip: {:x}",
    	access, cr2, reason, error_code, arch::cpu::current_slow().index, info.registers.rsi, info.registers.rsp, info.registers.rip);
}

extern fn default_handler(info: &Info, index: u8, error_code: usize)
{
    panic!("Unhandled interrupt: {} ({:#x})\n\nerrnr: {:#x}   cpu: {} rsi: {:x}  rsp: {:x}
rip: {:x}",
    	index, index, error_code, arch::cpu::current_slow().index, info.registers.rsi, info.registers.rsp, info.registers.rip);
}

#[allow(dead_code)]
#[repr(packed)]
#[derive(Copy, Clone)]
pub struct Gate {
	target_low: u16,
	segment_selector: u16,

/*
	unsigned int ist : 3;
	unsigned int reserved_0 : 5;
*/
	ist: u8,

/*
	unsigned int type : 4;
	unsigned int zero : 1;
	unsigned int privilege_level : 2;
	unsigned int present : 1;
*/
	misc: u8,


	target_medium: u16,
	target_high: u32,
	reserved_1: u32,
}

pub const GATE_DEF: Gate = Gate {
	target_low: 0,
	segment_selector: 0,
	ist: 0,
	misc: 0,
	target_medium: 0,
	target_high: 0,
	reserved_1: 0,
};

static mut IDT: [Gate; HANDLER_COUNT] = [GATE_DEF; HANDLER_COUNT];

pub unsafe fn ref_gate(index: u8) -> &'static mut Gate {
	&mut IDT[index as usize]
}

pub unsafe fn set_gate(index: u8, stub: unsafe extern fn (), ist: u8) {
	let target = stub as usize;

	let gate = &mut IDT[usize::from(index)];

	gate.target_low = target as u16;
	gate.target_medium = (target >> 16) as u16;
	gate.target_high = (target >> 32) as u32;
	gate.segment_selector = arch::segments::CODE_SEGMENT;
	gate.ist = ist;

	gate.misc = 0xE | 0b10000000; // present, type = 0xE
}

pub fn register_handler(index: u8, handler: Handler) {
	unsafe {
		HANDLERS[index as usize].store(handler as usize, SeqCst);
	}
}

pub unsafe fn load_idt() {
	let idt_ptr = arch::CPUPointer {
		limit: u16::coerce(size_of_val(&IDT)) - 1,
		base: offset(&IDT)
	};

    asm! {
        lidt {&idt_ptr => %*m};
    }

	arch::cpu::current_slow().arch.has_idt.store(true, SeqCst);
}

pub unsafe fn setup_fatal_handlers() {
	disable();

	set_gate(0x2, ISR_STUBS[0x2], 1);
	set_gate(0x8, ISR_STUBS[0x8], 2);
	set_gate(0xe, ISR_STUBS[0xe], 3);
}

pub unsafe fn initialize_idt() {
	setup_pics();

	for i in 0u8..0xFF {
		set_gate(i, ISR_STUBS[i as usize], 0);
	}

	set_gate(0xFF, spurious_irq, 0);

	for handler in HANDLERS.iter_mut() {
		handler.store(default_handler as usize, SeqCst);
	}

	register_handler(0x2, nmi_handler);
	register_handler(0xe, page_fault_handler);

	load_idt();
}
