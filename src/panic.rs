use crate::debug;
use crate::memory;
use crate::memory::Void;
use crate::tty;
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
macro_rules! panic {
	() => (crate::panic::kernel_panic("Unknown", 0));
	($reason:literal) => (crate::panic::kernel_panic($reason, 0));
	($reason:literal, $code:literal) => (crate::panic::kernel_panic($reason, $code));
}

/*
 * Initializes the TTY and prints a panic message.
 */
fn print_panic(reason: &str, code: u32) {
	tty::init();
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
pub fn kernel_panic(reason: &str, code: u32) -> ! {
	::cli!();
	print_panic(reason, code);
	unsafe {
		kernel_halt();
	}
}

/*
 * TODO doc
 */
pub fn kernel_panic_(reason: &str, code: u32, file: &str, line: u32) -> ! {
	::cli!();
	print_panic(reason, code);
	::println!("\n-- DEBUG --\nFile: {}; Line: {}", file, line);
	// TODO Print running process registers
	::println!();
	debug::print_callstack(::register_get!("ebp") as *const _, 8);
	unsafe {
		kernel_halt();
	}
}
