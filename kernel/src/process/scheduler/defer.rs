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

//! Deferred calls allow a processor to make another processor call a given function
//!
//! Deferred calls should be fast and **must not**:
//! - be blocking
//! - enter a critical section

use crate::{
	arch::core_id,
	process::scheduler::{CPU, per_cpu},
};
use core::{
	cell::UnsafeCell,
	hint, mem,
	ptr::NonNull,
	sync::atomic::{
		AtomicBool, AtomicUsize,
		Ordering::{Acquire, Relaxed, Release},
	},
};

// TODO implement async calls

struct DeferredCall {
	func: NonNull<dyn Fn()>,
	done: NonNull<AtomicBool>,
}

struct Slot {
	/// Set to `true` once ready to be read
	ready: AtomicBool,
	call: UnsafeCell<DeferredCall>,
}

/// Per-CPU, multiple-producer, single-consumer (MPSC) queue, for deferred calls
pub struct DeferredCallQueue {
	buf: [Slot; 64],

	count: AtomicUsize,
	head: AtomicUsize,
	tail: AtomicUsize,
}

impl DeferredCallQueue {
	/// Creates a new queue
	#[allow(clippy::new_without_default)]
	pub const fn new() -> Self {
		Self {
			#[allow(invalid_value)]
			buf: unsafe { mem::zeroed() },

			count: AtomicUsize::new(0),
			head: AtomicUsize::new(0),
			tail: AtomicUsize::new(0),
		}
	}

	/// Inserts a call on the queue
	fn insert(&self, call: DeferredCall) {
		let count = self.count.fetch_add(1, Acquire);
		// If the queue is full, wait
		if count > self.buf.len() {
			// TODO
		}
		// Allocate a slot
		let head = self.head.fetch_add(1, Acquire);
		let slot = &self.buf[head % self.buf.len()];
		// Store in slot (use volatile to prevent any reordering)
		unsafe {
			slot.call.get().write_volatile(call);
		}
		self.buf[head % self.buf.len()].ready.store(true, Release);
	}
}

/// Defers a call to `func` on the CPU `cpu`.
///
/// The function waits until the function has been executed before returning.
pub fn synchronous<F: 'static + Fn()>(cpu: u8, func: F) {
	// If this is the current core, execute immediately
	if cpu == core_id() {
		func();
		return;
	}
	// Get CPU
	let Some(cpu) = CPU.get(cpu as usize) else {
		return;
	};
	// Wrap to add a flag notifying when the function has returned
	// Push on queue
	let done = AtomicBool::new(false);
	let call = DeferredCall {
		func: NonNull::from_ref(&func),
		done: NonNull::from_ref(&done),
	};
	cpu.deferred_calls.insert(call);
	// Wait for the function to return
	while done.load(Acquire) {
		hint::spin_loop();
	}
}

/// Makes deferred calls in the current CPU's queue, if any
pub(super) fn consume() {
	let queue = &per_cpu().deferred_calls;
	// Limit spin count to avoid starvation in case the CPU is getting spammed
	for _ in 0..queue.buf.len() {
		// Dequeue an element
		let mut tail = queue.tail.load(Relaxed);
		let slot = &queue.buf[tail];
		let call = slot.ready.swap(false, Acquire);
		if !call {
			// The element hasn't been written yet, but we might get it and the following next time
			break;
		}
		// Perform call
		unsafe {
			let call = slot.call.get().read();
			call.func.as_ref()();
			call.done.as_ref().store(true, Release);
		}
		// Update tail and count
		tail = (tail + 1) % queue.buf.len();
		queue.tail.store(tail, Relaxed);
		queue.count.fetch_sub(1, Release);
	}
}
