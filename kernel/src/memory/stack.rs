/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! This module implements stack utility functions.

use crate::memory;
use core::{ffi::c_void, mem, ptr};
use utils::{errno::AllocResult, vec};

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
		let stack = vec![0; TMP_STACK_SIZE]?;
		let stack_top = (stack.as_ptr() as *mut c_void).add(TMP_STACK_SIZE);

		stack_switch_(stack_top, &mut f as *mut _ as _, func as *const _);
	}

	let ret_val = ptr::read(&f.ret_val).unwrap();

	// Avoid double free
	mem::forget(f);

	Ok(ret_val)
}
