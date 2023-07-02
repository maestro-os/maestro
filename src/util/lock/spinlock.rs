//! This module contains the Spinlock structure, which is considered as being a
//! low level feature.
//!
//! Unless for special cases, other locks should be used instead.

use core::arch::asm;

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
	/// Variable telling whether the spinlock is locked or not. This variable is
	/// 4 bytes wide to match the size of the register handling it (under x86).
	locked: i32,
}

impl Spinlock {
	/// Creates a new spinlock.
	pub const fn new() -> Self {
		Self {
			locked: 0,
		}
	}

	/// Locks the spinlock.
	#[inline(always)]
	pub fn lock(&mut self) {
		unsafe {
			asm!(
				"2:",
				"mov {x}, 1",
				"xchg [{lock}], {x}",
				"test {x}, {x}",
				"pause",
				"jnz 2b",
				x = out(reg) _,
				lock = in(reg) &mut self.locked,
			)
		}
	}

	/// Unlocks the spinlock.
	#[inline(always)]
	pub unsafe fn unlock(&mut self) {
		unsafe {
			asm!(
				"mov DWORD PTR [{lock}], 0",
				lock = in(reg) &mut self.locked,
			)
		}
	}
}
