//! This module implements stack utility functions.

use crate::{errno::AllocResult, memory, memory::malloc};
use core::{ffi::c_void, mem, ptr};

/// The size of a temporary stack in bytes.
const TMP_STACK_SIZE: usize = memory::PAGE_SIZE * 8;

extern "C" {
	/// Performs the stack switching for the given stack and closure to execute.
	///
	/// `s` is the `StackLambda` structure.
	fn stack_switch_(stack: *mut c_void, s: *mut c_void, f: *const c_void);
}

/// Structure storing a lambda to be executed on an alternate stack.
struct StackLambda<F: FnOnce() -> T, T> {
	/// The lambda to be called on the alternate stack.
	f: F,

	/// The return value.
	ret_val: Option<T>,
}

impl<F: FnOnce() -> T, T> StackLambda<F, T> {
	/// Performs the execution of the lambda on the alternate stack.
	extern "C" fn exec(&mut self) {
		let f = unsafe { ptr::read(&self.f) };

		self.ret_val = Some(f());
	}
}

/// Executes the given closure `f` while being on the given stack.
///
/// `stack` is the pointer to the beginning of the alternate stack.
///
/// If the given stack is `None`, the function allocates a temporary stack.
///
/// # Safety
///
/// If the stack `stack` is invalid, the behaviour is undefined.
///
/// When passing a closure to this function, the `move` keyword should be used in the case the
/// previous stack becomes unreachable. This keyword ensures that variables are captured by value
/// and not by reference, thus avoiding to create dangling references.
pub unsafe fn switch<F: FnOnce() -> T, T>(stack: Option<*mut c_void>, f: F) -> AllocResult<T> {
	let mut f = StackLambda {
		f,
		ret_val: None,
	};
	let func = StackLambda::<F, T>::exec;

	if let Some(stack) = stack {
		stack_switch_(stack, &mut f as *mut _ as _, func as *const _);
	} else {
		let stack = malloc::Alloc::<u8>::new_default(TMP_STACK_SIZE.try_into().unwrap())?;
		let stack_top = (stack.as_ptr() as *mut c_void).add(TMP_STACK_SIZE);

		stack_switch_(stack_top, &mut f as *mut _ as _, func as *const _);
	}

	let ret_val = ptr::read(&f.ret_val).unwrap();

	// Avoid double free
	mem::forget(f);

	Ok(ret_val)
}
