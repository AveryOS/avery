use memory::Addr;

static mut VGA: *mut u16 = 0xb8000 as *mut u16;

const SIZE_X: isize = 80;
const SIZE_Y: isize = 25;

const MIN_X: isize = 2;
const MIN_Y: isize = 1;
const MAX_X: isize = 78;
const MAX_Y: isize = 24;

const COLOR: u16 = (0 << 8) | (7 << 12);

static mut x: isize = MIN_X;
static mut y: isize = MIN_Y;

pub fn get_buffer_info() -> (Addr, usize) {
	unsafe { (VGA as Addr, (SIZE_X * SIZE_Y) as usize * size_of::<u16>()) }
}

pub fn set_buffer(addr: usize) {
	unsafe { VGA = addr as *mut u16 };
}

unsafe fn update_cursor()
{
	use arch::outb;

	let loc = y * SIZE_X + x;

	outb(0x3D4, 14);
	outb(0x3D5, (loc >> 8) as u8);
	outb(0x3D4, 15);
	outb(0x3D5, loc as u8);
}

pub fn scroll() {
	unsafe {
		for i in SIZE_X..(SIZE_X * (SIZE_Y - 1)) {
			*VGA.offset(i) = *VGA.offset(i + SIZE_X);
		}

		for i in 0..SIZE_X {
			*VGA.offset((SIZE_Y - 1) * SIZE_X + i) = ' ' as u16 | COLOR;
		}
	}
}

pub fn cls() {
	unsafe {
		for i in 0..(SIZE_X * SIZE_Y) {
			*VGA.offset(i) = ' ' as u16 | COLOR;
		}

		x = MIN_X;
		y = MIN_Y;

		update_cursor();
	}

}

pub fn newline() {
	unsafe {
		y += 1;
		x = MIN_X;

		if y >= MAX_Y {
			scroll();
			y = MAX_Y - 1;
		}

		update_cursor();
	}
}

pub fn putc(c: char) {
	unsafe {

		match c {
			'\n' => newline(),
			'\t' => {
				x = (x + 4) & !(4 - 1);

				if x >= MAX_X {
					newline();
				} else {
					update_cursor();
				}
			}
			_ => {
				if x >= MAX_X {
					newline();
				}

				*VGA.offset(y * SIZE_X + x) = c as u16 | COLOR;
				x += 1;
				update_cursor();
			}
		}

	}
}
