
use arch::{apic, outb, interrupts, IRQ};

pub const VECTOR: u8 = 34;

extern fn pit_interrupt(_: &interrupts::Info, _: u8, _: usize) {
    unsafe {
        apic::eoi();
    }
}

pub unsafe fn initialize(mut irq: IRQ) {
	interrupts::register_handler(VECTOR, pit_interrupt);

	irq.route(VECTOR, apic::local_id());

	let divisor: u32 = 1193182 / 200;

	outb(0x43, 0x34);
	outb(0x40, divisor as u8);
	outb(0x40, (divisor >> 8) as u8);
}
