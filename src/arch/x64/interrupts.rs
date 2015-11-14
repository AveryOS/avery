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

extern fn default_handler(info: &Info, index: u8, error_code: usize)
{
	let cr2: u64;

	unsafe {
	    asm! {
	    	[%rax => cr2]

	    	mov rax, cr2
	    }
	}

    panic!("Unhandled interrupt: {}\n\nerrnr: {:x}   cpu: {} rsi: {:x}  rsp: {:x}  cr2: {:x}
rip: {:x}",
    	index, error_code, arch::cpu::current_slow().index, info.registers.rsi, info.registers.rsp, cr2, info.registers.rip);
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

pub unsafe fn set_gate(index: u8, stub: unsafe extern fn ()) {
	let target = stub as usize;

	let gate = &mut IDT[index as usize];

	gate.target_low = target as u16;
	gate.target_medium = (target >> 16) as u16;
	gate.target_high = (target >> 32) as u32;
	gate.segment_selector = arch::segments::CODE_SEGMENT;

	gate.misc = 0xE | 0b10000000; // present, type = 0xE
}

pub fn register_handler(index: u8, handler: Handler) {
	unsafe {
		HANDLERS[index as usize].store(handler as usize, SeqCst);
	}
}

pub unsafe fn load_idt() {
	let idt_ptr = arch::CPUPointer {
		limit: size_of_val(&IDT) as u16 - 1,
		base: offset(&IDT)
	};

    asm! {
        lidt {&idt_ptr => %*m};
    }
}

pub unsafe fn initialize_idt() {
	setup_pics();

	for i in 0u8..0xFF {
		set_gate(i, ISR_STUBS[i as usize]);
	}

	set_gate(0xFF, spurious_irq);

	for handler in HANDLERS.iter_mut() {
		handler.store(default_handler as usize, SeqCst);
	}
	
	load_idt();
}
