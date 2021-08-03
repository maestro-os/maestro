//! TODO doc

use core::ffi::c_void;
use core::panic::PanicInfo;
use crate::panic;

/// The kernel's name.
pub const NAME: &str = "maestro";
/// Current kernel version.
pub const VERSION: &str = "1.0";

extern "C" {
	fn kernel_wait();
	fn kernel_loop() -> !;
	fn kernel_loop_reset(stack: *mut c_void) -> !;
	fn kernel_halt() -> !;
}

/// Makes the kernel wait for an interrupt, then returns.
/// This function enables interrupts.
pub fn wait() {
	unsafe {
		kernel_wait();
	}
}

/// Enters the kernel loop and processes every interrupts indefinitely.
pub fn enter_loop() -> ! {
	unsafe {
		kernel_loop();
	}
}

/// Resets the stack to the given value, then calls `enter_loop`.
/// The function is unsafe because the pointer passed in parameter might be invalid.
pub unsafe fn loop_reset(stack: *mut c_void) -> ! {
	kernel_loop_reset(stack);
}

/// Halts the kernel until reboot.
pub fn halt() -> ! {
	unsafe {
		kernel_halt();
	}
}

/// Called on Rust panic.
#[panic_handler]
fn panic(panic_info: &PanicInfo) -> ! {
	#[cfg(test)]
	if selftest::is_running() {
		println!("FAILED\n");
		println!("Error: {}\n", panic_info);
		halt();
	}

	if let Some(s) = panic_info.message() {
		panic::rust_panic(s);
	} else {
		crate::kernel_panic!("Rust panic (no payload)", 0);
	}
}
