//! This file handles kernel panics.
//! A kernel panic occurs when an error is raised that the kernel cannot recover from. This is an
//! undesirable state which requires to reboot the host machine.

use core::ffi::c_void;
use core::fmt::Arguments;
use core::fmt;
use crate::cpu;
#[cfg(config_debug_debug)]
use crate::debug;
use crate::tty;

/// Macro triggering a kernel panic.
/// `reason` is the reason of the kernel panic.
/// `code` is an optional special code provided with the reason.
#[macro_export]
macro_rules! kernel_panic {
	($($reason:tt)*) => {
		crate::panic::kernel_panic_(format_args!($($reason)*), file!(), line!(), column!())
	};
}

/// Initializes the TTY and prints a panic message.
/// `reason` is the reason of the kernel panic.
fn print_panic(reason: Arguments) {
	tty::init();
	crate::println!("--- KERNEL PANIC ---\n");
	crate::println!("Kernel has been forced to halt due to internal problem, sorry :/");
	crate::println!("Reason: {}", reason);
	crate::println!("CR2: {:p}\n", unsafe {
		cpu::cr2_get()
	} as *const c_void);
	crate::println!("If you believe this is a bug on the kernel side, please feel free to report \
it.");
}

/// Re-initializes the TTY, prints the panic message and halts the kernel.
/// `reason` is the reason of the kernel panic.
/// This function should not be called directly and should be used through the `kernel_panic` macro
/// only.
#[cfg(not(config_debug_debug))]
pub fn kernel_panic_(reason: Arguments, _file: &str, _line: u32, _col: u32) -> ! {
	crate::cli!();
	print_panic(reason);
	crate::halt();
}

/// Same as the release version, except the function also prints process's registers and the
/// kernel's callstack.
/// `file` is the file in which the kernel panic was triggerd.
/// `line` is the line at which the kernel panic was triggerd.
/// `column` is the column at which the kernel panic was triggerd.
/// This function should not be called directly and should be used through the `kernel_panic` macro
/// only.
#[cfg(config_debug_debug)]
pub fn kernel_panic_(reason: Arguments, file: &str, line: u32, col: u32) -> ! {
	crate::cli!();
	print_panic(reason);

	crate::println!("\n-- DEBUG --\nFile: {}; Line: {}; Column: {}", file, line, col);
	crate::println!();

	let ebp = unsafe {
		crate::register_get!("ebp") as *const _
	};
	debug::print_callstack(ebp, 8);

	crate::halt();
}

/// Initializes the TTY and prints a Rust panic message.
fn print_rust_panic<'a>(args: &'a fmt::Arguments<'a>) {
	tty::init();
	crate::println!("--- KERNEL PANIC ---\n");
	crate::println!("Kernel has been forced to halt due to internal problem, sorry :/");
	crate::println!("Reason: {}", args);
	crate::println!("CR2: {:p}\n", unsafe {
		cpu::cr2_get()
	} as *const c_void);
	crate::println!("If you believe this is a bug on the kernel side, please feel free to report
it.");
}

/// Handles a Rust panic.
#[cfg(not(config_debug_debug))]
pub fn rust_panic<'a>(args: &'a fmt::Arguments<'a>) -> ! {
	crate::cli!();
	print_rust_panic(args);

	crate::halt();
}

/// Same as the release version, except the function also prints the kernel's callstack.
#[cfg(config_debug_debug)]
pub fn rust_panic<'a>(args: &'a fmt::Arguments<'a>) -> ! {
	crate::cli!();
	print_rust_panic(args);
	crate::println!();

	let ebp = unsafe {
		crate::register_get!("ebp") as *const _
	};
	debug::print_callstack(ebp, 8);

	crate::halt();
}
