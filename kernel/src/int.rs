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

//! Interrupt callback register interface.

use crate::{
	arch::{
		end_of_interrupt,
		x86::{idt, idt::IntFrame},
	},
	memory::user::UserSlice,
	power::{halt, halting},
	process::scheduler::{alter_flow, cpu::CPU, preempt_check_resched},
	rand,
};
use core::{array, cell::UnsafeCell, hint::unlikely, ptr::NonNull};
use utils::{boxed::Box, bytes::as_bytes, errno::AllocResult};

type CallbackInner = dyn FnMut(u32, u32, &mut IntFrame, u8);
/// A callback to handle an interruption
pub type Callback = Box<CallbackInner>;

/// Per-CPU callback list, stored in [`PerCpu`].
pub struct CallbackList([UnsafeCell<Option<Callback>>; idt::ENTRIES_COUNT]);

impl Default for CallbackList {
	fn default() -> Self {
		Self(array::from_fn(|_| UnsafeCell::new(None)))
	}
}

/// Structure used to detect whenever the object owning the callback is
/// destroyed, allowing to unregister it automatically.
#[must_use]
pub struct CallbackHook {
	/// The CPU core the callback is bound to
	cpu: u32,
	/// The ID of the interrupt the callback is bound to
	id: u32,
	/// Pointer to the callback, used for removal
	callback: NonNull<CallbackInner>,
}

impl Drop for CallbackHook {
	fn drop(&mut self) {
		// Remove the callback
		CPU[self.cpu as usize].int_callbacks.0[self.id as usize] = None;
	}
}

/// Registers a callback for the interrupt ID `id` on the current CPU core.
///
/// The latest registered callback is executed last.
///
/// `callback` arguments:
/// - `id` is the id of the interrupt.
/// - `code` is an optional code associated with the interrupt. If no code is given, the value is
///   `0`.
/// - `regs` the values of the registers when the interruption was triggered.
/// - `ring` tells the ring at which the code was running.
///
/// If an allocation fails, the function shall return an error.
///
/// If the provided ID is invalid, the function returns `None`.
///
/// On success, the function returns a hook that unregisters the callback on drop.
pub fn register_callback<F: 'static + FnMut(u32, u32, &mut IntFrame, u8)>(
	id: u32,
	callback: F,
) -> AllocResult<Option<CallbackHook>> {
	let Some(callbacks) = CALLBACKS.get(id as usize) else {
		return Ok(None);
	};
	let callback = Box::new(callback)?;
	let ptr = NonNull::from(Box::as_ref(&callback));
	let mut vec = callbacks.lock();
	vec.push(callback)?;
	Ok(Some(CallbackHook {
		cpu,
		id,
		callback: ptr,
	}))
}

/// Called whenever an interruption is triggered.
///
/// `frame` is the stack frame of the interruption, with general purpose registers saved.
#[unsafe(no_mangle)]
extern "C" fn interrupt_handler(frame: &mut IntFrame) {
	if unlikely(halting()) {
		halt();
	}
	// Ignore page faults to avoid a deadlock (might occur when writing entropy to userspace on
	// non-mapped page)
	if frame.int != 0xe {
		// Feed entropy pool
		let mut pool = rand::ENTROPY_POOL.lock();
		if let Some(pool) = &mut *pool {
			let buf = unsafe { UserSlice::from_slice(as_bytes(frame)) };
			let _ = pool.write(buf);
		}
	}
	let id = frame.int as u32;
	let ring = (frame.cs & 0b11) as u8;
	let code = frame.code as u32;
	// Call corresponding callbacks
	{
		let mut callbacks = CALLBACKS[id as usize].lock();
		for c in callbacks.iter_mut() {
			c(id, code, frame, ring);
		}
	}
	// If not a hardware exception, send EOI
	if let Some(irq) = id.checked_sub(32) {
		end_of_interrupt(irq as _);
	}
	alter_flow(ring, frame);
	preempt_check_resched();
}
