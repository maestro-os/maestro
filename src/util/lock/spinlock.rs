//! This module contains the Spinlock structure, which is considered as being a
//! low level feature.
//!
//! Unless for special cases, other locks should be used instead.

use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;

/// A spinlock is a lock that is used to prevent a specific piece of code from
/// being accessed by more than one thread at a time.
///
/// It works by storing a value telling whether a thread is already in that piece of code.
///
/// To avoid race conditions, the implementation uses an atomic exchange instruction to
/// check/lock the structure. If a threads tries to lock the structure while
/// already being locked, the thread shall wait in a loop (spin) until the
/// structure is unlocked.
///
/// Special attention must be aimed toward the usage of this structure since it
/// can easily result in deadlocks if misused.
pub struct Spinlock {
	locked: AtomicBool,
}

impl Spinlock {
	/// Creates a new spinlock.
	pub const fn new() -> Self {
		Self {
			locked: AtomicBool::new(false),
		}
	}

	/// Locks the spinlock.
	#[inline(always)]
	pub fn lock(&mut self) {
		while self.locked.swap(true, Ordering::Acquire) {}
	}

	/// Unlocks the spinlock.
	#[inline(always)]
	pub unsafe fn unlock(&mut self) {
		self.locked.store(false, Ordering::Release);
	}
}
