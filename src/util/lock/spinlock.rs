//! This module contains the Spinlock structure, which is considered as being a low level feature.
//! Unless for special cases, other locks should be used instead.

extern "C" {
	pub fn spin_lock(lock: *mut i32);
	pub fn spin_unlock(lock: *mut i32);
}

/// A spinlock is a lock that is used to prevent a specific piece of code from being accessed by
/// more than one thread at a time. It works by storing a value telling whether a thread is already
/// in that piece of code. To avoid race conditions, the implementation uses an atomic exchange
/// instruction to check/lock the structure.
/// If a threads tries to lock the structure while already being locked, the thread shall wait in
/// a loop (spin) until the structure is unlocked.
///
/// Special attention must be aimed toward the usage of this structure since it can easily result
/// in deadlocks if misused.
pub struct Spinlock {
	/// Variable telling whether the spinlock is locked or not. This variable is 4 bytes wide to
	/// match the size of the register handling it (under x86).
	locked: i32,
}

impl Spinlock {
	/// Creates a new spinlock.
	pub const fn new() -> Self {
		Self {
			locked: 0,
		}
	}

	/// Tells whether the spinlock is already locked. This function should not be called to check
	/// if the spinlock is ready to be locked before locking it, since it may cause race
	/// conditions. In this case, prefer using `lock` directly.
	pub fn is_locked(&self) -> bool {
		self.locked != 0
	}

	/// Wrapper for `spin_lock`. Locks the spinlock.
	pub fn lock(&mut self) {
		unsafe {
			spin_lock(&mut self.locked);
		}
	}

	/// Wrapper for `spin_unlock`. Unlocks the spinlock.
	pub unsafe fn unlock(&mut self) {
		spin_unlock(&mut self.locked);
	}
}
