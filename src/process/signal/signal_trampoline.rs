//! This file implements the signal handler trampoline.
//! The trampoline is using the same stack as the normal process execution. However, the System V
//! ABI defines a region of the stack located after the allocated portion which is called the red
//! zone. This region must not be clobbered, thus the kernel adds an offset on the stack
//! corresponding to the size of the red zone.
//!
//! When the signal handler returns, the process returns directly to execution.

use core::arch::asm;
use core::ffi::c_void;
use core::mem::transmute;

/// The signal handler trampoline. The process resumes to this function when it received a signal.
/// Thus, this code is executed in userspace.
/// When the process finished handling the signal, it calls the `sigreturn` system call in order to
/// tell the kernel to resume normal execution.
///
/// `handler` is a pointer to the handler function for the signal.
/// `sig` is the signal number.
#[no_mangle]
pub extern "C" fn signal_trampoline(handler: *const c_void, sig: i32) -> ! {
	// Calling the signal handler
	unsafe {
		let handler = transmute::<*const c_void, unsafe extern "C" fn(i32)>(handler);
		handler(sig);
	}

	// Calling `sigreturn` to end signal handling.
	unsafe {
		asm!("mov eax, 0x077\nint 0x80"); // TODO Adapt the system call number to the arch
	}

	// Trying to kill the process if reaching this for some reason
	unsafe {
		#[allow(deref_nullptr)]
		*(0x0 as *mut u32) = 42;
	}
	loop {}
}
