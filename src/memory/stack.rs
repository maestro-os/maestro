/// This module implements stack utility functions.

use crate::util::boxed::Box;
use crate::errno::Errno;
use core::ffi::c_void;

extern "C" {
	fn stack_switch_(stack: *mut c_void, func_ptr: *const c_void, data: *const c_void);
}

/// TODO doc
#[no_mangle]
extern "C" fn stack_switch_in(func_ptr: *const c_void, data: *const c_void) {
	let f = unsafe { // Call to unsafe function
		core::mem::transmute::<*const c_void, fn(*const c_void)>(func_ptr)
	};
	f(data);
}

/// Executes the given closure `f` while being on the given stack. `stack` is the pointer to the
/// beginning of the new stack. After execution, the function restores the previous stack.
/// `data` is the data to pass on the temporary stack.
pub unsafe fn stack_switch<T>(stack: *mut c_void, f: fn(*const c_void), data: T)
	-> Result::<(), Errno> {
	let data_box = Box::new(data)?;
	let data_ptr = data_box.as_ptr();
	stack_switch_(stack, f as _, data_ptr as _);
	Ok(())
}
