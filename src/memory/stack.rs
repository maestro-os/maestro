//! This module implements stack utility functions.

use core::ffi::c_void;
use core::mem::MaybeUninit;
use core::ptr::copy_nonoverlapping;

extern "C" {
	/// Performs the stack switching for the given stack and closure to execute.
	fn stack_switch_(stack: *mut c_void, func_ptr: *mut c_void, f: *const c_void)
		-> *const c_void;
}

/// Structure storing a lambda to be executed on an alternate stack.
struct StackLambda<T, F: FnMut() -> T> {
	/// The lambda to be called on the alternate stack.
	f: F,
}

impl<T, F: FnMut() -> T> StackLambda<T, F> {
	/// Performs the execution of the lambda on the alternate stack.
	extern "C" fn stack_switch_in(&mut self) -> T {
		(self.f)()
	}
}

/// Executes the given closure `f` while being on the given stack. `stack` is the pointer to the
/// beginning of the alternate stack.
///
/// # Safety
///
/// If the stack `stack` is invalid, the behaviour is undefined.
pub unsafe fn switch<T, F: FnMut() -> T>(stack: *mut c_void, f: F) -> T {
	let f = StackLambda {
		f,
	};
	let func = StackLambda::<T, F>::stack_switch_in;

	let result_ptr = stack_switch_(stack, &f as *const _ as _, func as *mut _);

	let mut result = MaybeUninit::<T>::uninit();
	copy_nonoverlapping(result_ptr as *const T, result.assume_init_mut(), 1);
	result.assume_init()
}
