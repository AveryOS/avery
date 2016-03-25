
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

	let divisor = u16::coerce(1193182u32 / 20);

    println!("PIT divisor {}", divisor);

	outb(0x43, 0x34);
	outb(0x40, divisor.split().0);
	outb(0x40, divisor.split().1);
}
