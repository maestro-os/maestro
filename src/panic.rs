//use crate::debug;
use crate::memory;
use crate::memory::Void;
//use crate::tty;
use kernel_halt;

/*
 * This file handles kernel panics.
 * A kernel panic occurs when an error is raised that the kernel cannot recover from. This is an undesirable state which
 * requires to reboot the host machine.
 */

/*
 * Macro triggering a kernel panic.
 */
#[macro_export]
macro_rules! kernel_panic {
	() => {
		crate::panic::kernel_panic_("Unknown", 0, file!(), line!(), column!())
	};
	($reason:expr) => {
		crate::panic::kernel_panic_($reason, 0, file!(), line!(), column!())
	};
	($reason:expr, $code:expr) => {
		crate::panic::kernel_panic_($reason, $code, file!(), line!(), column!())
	};
}

/*
 * Initializes the TTY and prints a panic message.
 */
fn print_panic(reason: &str, code: u32) {
	::println!("--- KERNEL PANIC ---\n");
	::println!("Kernel has been forced to halt due to internal problem, sorry :/");
	::println!("Reason: {}", reason);
	::println!("Error code: {}", code);
	::println!("CR2: {:p}\n", unsafe { memory::vmem::cr2_get() } as *const Void);
	::println!("If you believe this is a bug on the kernel side, please feel free to report it.");
}

/*
 * TODO doc
 */
#[cfg(kernel_mode = "release")]
pub fn kernel_panic_(reason: &str, code: u32, _file: &str, _line: u32, _col: u32) -> ! {
	::cli!();
	tty::init();
	print_panic(reason, code);
	unsafe {
		kernel_halt();
	}
}

/*
 * TODO doc
 */
#[cfg(kernel_mode = "debug")]
pub fn kernel_panic_(reason: &str, code: u32, file: &str, line: u32, col: u32) -> ! {
	::cli!();
	//tty::init();
	print_panic(reason, code);
	::println!("\n-- DEBUG --\nFile: {}; Line: {}; Column: {}", file, line, col);
	// TODO Print running process registers
	::println!();
	//let ebp = unsafe { ::register_get!("ebp") as *const _ };
	//debug::print_callstack(ebp, 8);
	unsafe {
		kernel_halt();
	}
}
