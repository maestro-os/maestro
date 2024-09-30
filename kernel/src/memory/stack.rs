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

//! Stack utility functions.

use core::{
	arch::asm,
	ffi::c_void,
	mem,
	mem::{size_of, MaybeUninit},
	ptr,
};

/// A closure to be executed on an alternate stack.
struct StackInfo<F: FnOnce() -> T, T> {
	/// The lambda to be called on the alternate stack.
	f: F,
	/// The return value.
	ret_val: MaybeUninit<T>,
}

impl<F: FnOnce() -> T, T> StackInfo<F, T> {
	/// Performs the execution of the lambda on the alternate stack.
	extern "C" fn exec(&mut self) {
		let f = unsafe { ptr::read(&self.f) };
		self.ret_val.write(f());
	}
}

/// Executes the given closure `f` while being on the given stack.
///
/// `stack` is the pointer to the beginning of the alternate stack.
///
/// # Safety
///
/// If the stack `stack` is invalid, the behaviour is undefined.
///
/// When passing a closure to this function, the `move` keyword should be used in the case the
/// previous stack becomes unreachable. This keyword ensures that variables are captured by value
/// and not by reference, thus avoiding to create dangling references.
pub unsafe fn switch<F: FnOnce() -> T, T>(stack: *mut c_void, f: F) -> T {
	debug_assert!(stack.is_aligned_to(size_of::<usize>()));
	let mut f = StackInfo {
		f,
		ret_val: MaybeUninit::uninit(),
	};
	let func = StackInfo::<F, T>::exec;
	asm!(
		// Save stack
		"mov {esp_stash}, esp",
		"mov {ebp_stash}, ebp",
		// Set new stack
		"mov esp, {stack}",
		"xor ebp, ebp",
		// Call execution function
		"push {f}",
		"call {func}",
		// Restore previous stack
		"mov esp, {esp_stash}",
		"mov ebp, {ebp_stash}",
		esp_stash = out(reg) _,
		ebp_stash = out(reg) _,
		stack = in(reg) stack,
		f = in(reg) &mut f,
		func = in(reg) func
	);
	let StackInfo {
		f,
		ret_val,
	} = f;
	// Avoid double free
	mem::forget(f);
	ret_val.assume_init()
}
