//! This module implements stack utility functions.

use core::ffi::c_void;
use core::mem::transmute;

extern "C" {
	/// Performs the stack switching for the given stack and closure to execute.
	fn stack_switch_(stack: *mut c_void, func_ptr: *const c_void, data: *mut c_void);
}

/// Performs the execution of the given function `f`.
#[no_mangle]
extern "C" fn stack_switch_in(func_ptr: *const c_void, data: *mut c_void) {
	let f = unsafe {
		transmute::<*const c_void, fn(*const c_void)>(func_ptr)
	};
	f(data);
}

/// Executes the given closure `f` while being on the given stack. `stack` is the pointer to the
/// beginning of the new stack. `data` is passed as an argument to `f`.
pub unsafe fn switch<T>(stack: *mut c_void, f: fn(*mut c_void), data: *mut T) {
	stack_switch_(stack, f as _, data as _);
}
