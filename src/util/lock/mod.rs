//! This module implements locks, useful to prevent race conditions in
//! multithreaded code for example.
//!
//! Mutual exclusion is used to protect data from concurrent access.
//!
//! A `Mutex` allows to ensure that one, and only thread accesses the data stored
//! into it at the same time. Preventing race conditions. They usually work
//! using spinlocks.
//!
//! One particularity with kernel development is that multi-threading is not the
//! only way to get concurrency issues. Another factor to take into account is
//! that fact that an interruption may be triggered at any moment while
//! executing the code unless disabled. For this reason, mutexes in the kernel
//! are equiped with an option allowing to disable interrupts while being
//! locked.
//!
//! If an exception is raised while a mutex that disables interruptions is
//! acquired, the behaviour is undefined.

pub mod spinlock;

use crate::idt;
use crate::util::lock::spinlock::Spinlock;
use core::cell::UnsafeCell;
use core::ops::Deref;
use core::ops::DerefMut;

/// Structure representing the saved state of interruptions for the current
/// thread.
struct State {
	/// The number of currently locked mutexes that disable interruptions.
	ref_count: usize,

	/// Tells whether interruptions were enabled before locking mutexes.
	enabled: bool,
}

// TODO When implementing multicore, use an array. One element per core
/// Saved state of interruptions for the current thread.
///
/// This variable doesn't require synchonization since interruptions are always
/// disabled when it is accessed.
static mut INT_DISABLE_REFS: State = State {
	ref_count: 0,

	enabled: false,
};

/// Type used to declare a guard meant to unlock the associated `Mutex` at the
/// moment the execution gets out of the scope of its declaration.
pub struct MutexGuard<'a, T: ?Sized, const INT: bool> {
	/// The mutex associated to the guard
	mutex: &'a Mutex<T, INT>,
}

impl<T: ?Sized, const INT: bool> Deref for MutexGuard<'_, T, INT> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		unsafe { self.mutex.get_payload() }
	}
}

impl<T: ?Sized, const INT: bool> DerefMut for MutexGuard<'_, T, INT> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe { self.mutex.get_mut_payload() }
	}
}

unsafe impl<T: ?Sized + Sync, const INT: bool> Sync for MutexGuard<'_, T, INT> {}

impl<T: ?Sized, const INT: bool> Drop for MutexGuard<'_, T, INT> {
	fn drop(&mut self) {
		unsafe {
			self.mutex.unlock();
		}
	}
}

/// The inner structure of the `Mutex` structure.
struct MutexIn<T: ?Sized, const INT: bool> {
	/// The spinlock for the underlying data.
	spin: Spinlock,

	/// The data associated to the mutex.
	data: T,
}

/// The object wrapped in a `Mutex` can be accessed by only one thread at a time.
///
/// The `INT` generic parameter tells whether interrupts are allowed while
/// the mutex is locked. The default value is `true`.
pub struct Mutex<T: ?Sized, const INT: bool = true> {
	/// An unsafe cell to the inner structure of the Mutex.
	inner: UnsafeCell<MutexIn<T, INT>>,
}

impl<T, const INT: bool> Mutex<T, INT> {
	/// Creates a new Mutex with the given data to be owned.
	pub const fn new(data: T) -> Self {
		Self {
			inner: UnsafeCell::new(MutexIn {
				spin: Spinlock::new(),

				data,
			}),
		}
	}
}

impl<T: ?Sized, const INT: bool> Mutex<T, INT> {
	/// Returns an immutable reference to the payload.
	///
	/// # Safety
	///
	/// When using this function one, must be careful that another thread cannot access the
	/// resource simultaneously, which would result in an undefined behaviour.
	pub unsafe fn get_payload(&self) -> &T {
		&(*self.inner.get()).data
	}

	/// Returns a mutable reference to the payload.
	///
	/// # Safety
	///
	/// When using this function one, must be careful that another thread cannot access the
	/// resource simultaneously, which would result in an undefined behaviour.
	pub unsafe fn get_mut_payload(&self) -> &mut T {
		&mut (*self.inner.get()).data
	}

	/// Locks the mutex.
	///
	/// If the mutex is already locked, the thread shall wait until it becomes available.
	///
	/// The function returns a `MutexGuard` associated with the `Mutex`. When dropped, the mutex is
	/// unlocked.
	pub fn lock(&self) -> MutexGuard<T, INT> {
		let inner = unsafe {
			// Safe because using the spinlock later
			&mut *self.inner.get()
		};

		if !INT {
			let state = idt::is_interrupt_enabled();

			// Here is assumed that no interruption will change eflags' INT. Which could
			// cause a race condition

			// Disabling interrupts before locking to ensure no interrupt will occure while
			// locking
			crate::cli!();

			inner.spin.lock();

			// Updating the current thread's state
			// Safe because interrupts are disabled and the value can be accessed only by
			// the current core
			unsafe {
				if INT_DISABLE_REFS.ref_count == 0 {
					INT_DISABLE_REFS.enabled = state;
				}
				INT_DISABLE_REFS.ref_count += 1;
			}
		} else {
			inner.spin.lock();
		}

		MutexGuard {
			mutex: self,
		}
	}

	/// Unlocks the mutex. This function shouldn't be used directly since it is called when the
	/// mutex guard is dropped.
	///
	/// # Safety
	///
	/// If the mutex is not locked, the behaviour is undefined.
	///
	/// Unlocking the mutex while the resource is being used may result in concurrent access.
	pub unsafe fn unlock(&self) {
		let inner = &mut (*self.inner.get());

		if !INT {
			// Updating references count
			INT_DISABLE_REFS.ref_count -= 1;
			let state = if INT_DISABLE_REFS.ref_count == 0 {
				INT_DISABLE_REFS.enabled
			} else {
				false
			};

			// The state to restore
			inner.spin.unlock();

			// Restoring interrupts state after unlocking
			if state {
				crate::sti!();
			} else {
				crate::cli!();
			}
		} else {
			inner.spin.unlock();
		}
	}
}

unsafe impl<T, const INT: bool> Sync for Mutex<T, INT> {}

/// Type alias on `Mutex` representing a mutex which blocks interrupts.
pub type IntMutex<T> = Mutex<T, false>;
