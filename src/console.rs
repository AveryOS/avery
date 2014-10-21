use core::fmt::{FormatWriter, Arguments, FormatError};

pub use arch;

struct ScreenWriter;

impl FormatWriter for ScreenWriter {
    fn write(&mut self, bytes: &[u8]) -> Result<(), FormatError> {
		for c in bytes.iter() {
			arch::console::putc(*c as char);
		}

		Ok(())
    }
}

macro_rules! print(
    ($fmt:expr $($arg:tt)*) => (
        format_args!(::console::print_args, $fmt $($arg)*)
    )
)

macro_rules! println(
    ($fmt:expr $($arg:tt)*) => (
        format_args!(::console::print_args, concat!($fmt, "\n") $($arg)*)
    )
)

macro_rules! panic(
    ($fmt:expr $($arg:tt)*) => (
        format_args!(::console::panic, concat!($fmt, "\n") $($arg)*)
    )
)

pub fn print_args(args: &Arguments) {
	assert!(ScreenWriter.write_fmt(args).is_ok());
}

pub fn panic(args: &Arguments) -> ! {
	print!("Panic: ")
	assert!(ScreenWriter.write_fmt(args).is_ok());
	arch::halt();
}

#[lang = "begin_unwind"]
extern fn begin_unwind(args: &Arguments,
                       file: &str,
                       line: uint) -> ! {
    panic!("Error (begin_unwind): {} in {}:{}", args, file, line);
}

#[lang = "stack_exhausted"]
extern fn stack_exhausted()
{
    panic!("Stack overflow, which should not happen");
}

#[lang = "eh_personality"] extern fn eh_personality()
{
    panic!("Exceptions not supported");
}

#[lang = "fail_fmt"]
extern fn fail_fmt(fmt: &Arguments, file: &'static str, line: uint) -> ! {
    panic!("Error (fail_fmt)\nMsg: {}\nLoc: {}:{}", fmt, file, line);
}
