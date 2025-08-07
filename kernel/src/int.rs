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
		x86::{idt, idt::IntFrame}
	},
	memory::user::UserSlice,
	rand,
	process::scheduler::{alter_flow, may_schedule},
	sync::mutex::IntMutex,
};
use core::ptr;
use utils::{bytes::as_bytes, collections::vec::Vec, errno::AllocResult};

/// The list of interrupt error messages ordered by index of the corresponding
/// interrupt vector.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
static ERROR_MESSAGES: &[&str] = &[
	"Divide-by-zero Error",
	"Debug",
	"Non-maskable Interrupt",
	"Breakpoint",
	"Overflow",
	"Bound Range Exceeded",
	"Invalid Opcode",
	"Device Not Available",
	"Double Fault",
	"Coprocessor Segment Overrun",
	"Invalid TSS",
	"Segment Not Present",
	"Stack-Segment Fault",
	"General Protection Fault",
	"Page Fault",
	"Unknown",
	"x87 Floating-Point Exception",
	"Alignment Check",
	"Machine Check",
	"SIMD Floating-Point Exception",
	"Virtualization Exception",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Security Exception",
	"Unknown",
];

/// The action to execute after the interrupt handler has returned.
pub enum CallbackResult {
	/// Executes remaining callbacks for the interrupt.
	///
	/// If this is the last callback to be executed, the current context is yielded.
	Continue,
	/// Makes the kernel panic with a message corresponding to the interruption.
	Panic,
}

/// A callback to handle an interruption.
///
/// Arguments:
/// - `id` is the id of the interrupt.
/// - `code` is an optional code associated with the interrupt. If no code is given, the value is
///   `0`.
/// - `regs` the values of the registers when the interruption was triggered.
/// - `ring` tells the ring at which the code was running.
///
/// The return value tells which action to perform next.
pub type Callback = fn(u32, u32, &mut IntFrame, u8) -> CallbackResult;

/// Structure used to detect whenever the object owning the callback is
/// destroyed, allowing to unregister it automatically.
#[must_use]
pub struct CallbackHook {
	/// The id of the interrupt the callback is bound to.
	id: u32,
	/// The pointer of the callback.
	callback: Callback,
}

impl Drop for CallbackHook {
	fn drop(&mut self) {
		// Remove the callback
		let mut vec = CALLBACKS[self.id as usize].lock();
		let i = vec
			.iter()
			.enumerate()
			.find(|(_, c)| ptr::fn_addr_eq(**c, self.callback))
			.map(|(i, _)| i);
		if let Some(i) = i {
			vec.remove(i);
		}
	}
}

/// The default value for `CALLBACKS`.
#[allow(clippy::declare_interior_mutable_const)]
const CALLBACKS_INIT: IntMutex<Vec<Callback>> = IntMutex::new(Vec::new());
/// List containing vectors that store callbacks for every interrupt watchdogs.
static CALLBACKS: [IntMutex<Vec<Callback>>; idt::ENTRIES_COUNT as _] =
	[CALLBACKS_INIT; idt::ENTRIES_COUNT as _];

/// Registers the given callback and returns a reference to it.
///
/// The latest registered callback is executed last. Thus, callback that are registered first can
/// prevent next callbacks from being executed.
///
/// Arguments:
/// - `id` is the id of the interrupt to watch.
/// - `callback` is the callback to register.
///
/// If an allocation fails, the function shall return an error.
///
/// If the provided ID is invalid, the function returns `None`.
pub fn register_callback(id: u32, callback: Callback) -> AllocResult<Option<CallbackHook>> {
	let Some(callbacks) = CALLBACKS.get(id as usize) else {
		return Ok(None);
	};
	let mut vec = callbacks.lock();
	vec.push(callback)?;
	Ok(Some(CallbackHook {
		id,
		callback,
	}))
}

/// Called whenever an interruption is triggered.
///
/// `frame` is the stack frame of the interruption, with general purpose registers saved.
#[unsafe(no_mangle)]
extern "C" fn interrupt_handler(frame: &mut IntFrame) {
	may_schedule();
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
	let callbacks = &CALLBACKS[id as usize];
	let mut i = 0;
	loop {
		// Not putting this in a loop's condition to ensure it is dropped at each turn
		let Some(callback) = callbacks.lock().get(i).cloned() else {
			break;
		};
		i += 1;
		let res = callback(id, code, frame, ring);
		match res {
			CallbackResult::Continue => {}
			CallbackResult::Panic => {
				let error = ERROR_MESSAGES.get(id as usize).unwrap_or(&"Unknown");
				panic!("{error}, code: {code:x}");
			}
		}
	}
	// If not a hardware exception, send EOI
	if let Some(irq) = id.checked_sub(ERROR_MESSAGES.len() as u32) {
		end_of_interrupt(irq as _);
	}
	alter_flow(ring, frame);
}
