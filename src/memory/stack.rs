//! This module implements stack utility functions.

use core::ffi::c_void;

extern "C" {
	/// Performs the stack switching for the given stack and closure to execute.
	/// `s` is the StackLambda structure.
	fn stack_switch_(stack: *mut c_void, s: *mut c_void, f: *const c_void);
}

/// Structure storing a lambda to be executed on an alternate stack.
struct StackLambda<F: FnMut()> {
	/// The lambda to be called on the alternate stack.
	f: F,
}

impl<F: FnMut()> StackLambda<F> {
	/// Performs the execution of the lambda on the alternate stack.
	extern "C" fn exec(&mut self) {
		(self.f)()
	}
}

/// Executes the given closure `f` while being on the given stack. `stack` is the pointer to the
/// beginning of the alternate stack.
///
/// # Safety
///
/// If the stack `stack` is invalid, the behaviour is undefined.
pub unsafe fn switch<F: FnMut()>(stack: *mut c_void, f: F) {
	let mut f = StackLambda {
		f,
	};
	let func = StackLambda::<F>::exec;

	stack_switch_(stack, &mut f as *mut _ as _, func as *const _);
}
