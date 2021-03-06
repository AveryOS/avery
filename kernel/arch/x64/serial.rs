use arch::{inb, outb};

const PORT: u16 = 0x3f8; /* COM1 */

static mut INIT: bool = false;

unsafe fn initialize() {
	if INIT {
		return;
	}

	outb(PORT + 1, 0x00);    // Disable all interrupts
	outb(PORT + 3, 0x80);    // Enable DLAB (set baud rate divisor)
	outb(PORT   , 0x03);    // Set divisor to 3 (lo byte) 38400 baud
	outb(PORT + 1, 0x00);    //                  (hi byte)
	outb(PORT + 3, 0x03);    // 8 bits, no parity, one stop bit
	outb(PORT + 2, 0xC7);    // Enable FIFO, clear them, with 14-byte threshold
	outb(PORT + 4, 0x0B);    // IRQs enabled, RTS/DSR set

	INIT = true;
}

pub fn writeb(c: u8) {
	unsafe {
		initialize();
		while inb(PORT + 5) & 0x20 == 0 {}
		outb(PORT, c);
	}
}
