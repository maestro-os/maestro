//! A signal handler trampoline is the function that handles returning from a signal handler.
//!
//! The trampoline is using the same stack as the normal process execution.
//!
//! However, the **System V ABI** defines a region of the stack located after the
//! allocated portion which is called the **redzone**. This region must not be
//! clobbered, thus the kernel adds an offset on the stack corresponding to the
//! size of the redzone.
//!
//! When the signal handler returns, the process returns directly to execution.

use core::{arch::asm, ffi::c_void, mem::transmute};

/// The signal handler trampoline.
///
/// The process resumes to this function when it received a signal.
/// Thus, this code is executed in userspace.
///
/// When the process finished handling the signal, it calls the `sigreturn`
/// system call in order to tell the kernel to resume normal execution.
///
/// Arguments:
/// - `handler` is a pointer to the handler function for the signal.
/// - `sig` is the signal number.
#[no_mangle]
pub extern "C" fn signal_trampoline(handler: *const c_void, sig: i32) -> ! {
	// Calling the signal handler
	unsafe {
		let handler = transmute::<*const c_void, unsafe extern "C" fn(i32)>(handler);
		handler(sig);
	}

	// Calling `sigreturn` to end signal handling.
	unsafe {
		asm!("mov eax, 0x077\nint 0x80");
	}

	unreachable!();
}
