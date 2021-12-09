//! This module contains the Mutex and MutexGuard structure.
//!
//! Mutual exclusion is used to protect data from concurrent access.
//! A Mutex allows to ensure that one, and only thread accesses the data stored into it at the same
//! time. Preventing race conditions. They usually work using spinlocks.
//!
//! One particularity with kernel development is that multi-threading is not the only way to get
//! concurrency issues. Another factor to take into account is that fact that an interruption may
//! be triggered at any moment while executing the code unless disabled. For this reason, mutexes
//! in the kernel are equiped with an option allowing to disable interrupts while being locked.

use core::cell::UnsafeCell;
use core::marker::PhantomData;
use crate::idt;
use crate::util::lock::spinlock::Spinlock;

// TODO Add a deadlock detection mechanism (and mention it into the documentation)

/// Type used to declare a guard meant to unlock the associated Mutex at the moment the execution
/// gets out of the scope of its declaration. This structure is useful to ensure that the mutex
/// doesen't stay locked after the exectution of a function ended.
pub struct MutexGuard<'a, T: ?Sized> {
	/// The mutex associated to the guard
	mutex: &'a Mutex<T>,

	_data: PhantomData<T>,
}

impl<'a, T: ?Sized> MutexGuard<'a, T> {
	/// Creates an instance of MutexGuard for the given mutex `mutex`.
	fn new(mutex: &'a Mutex<T>) -> Self {
		Self {
			mutex,

			_data: PhantomData,
		}
	}

	/// Returns an immutable reference to the data owned by the associated Mutex.
	pub fn get(&self) -> &T {
		unsafe {
			self.mutex.get_payload()
		}
	}

	/// Returns a mutable reference to the data owned by the associated Mutex.
	pub fn get_mut(&mut self) -> &mut T {
		unsafe {
			self.mutex.get_mut_payload()
		}
	}

	/// Unlocks the Mutex.
	pub fn unlock(self) {}
}

impl<'a, T: ?Sized> Drop for MutexGuard<'a, T> {
	fn drop(&mut self) {
		unsafe {
			self.mutex.unlock();
		}
	}
}

/// The inner structure of the Mutex structure.
pub struct MutexIn<T: ?Sized> {
	/// The spinlock for the underlying data.
	spin: Spinlock,
	/// Tells whether locking disabled interrupts.
	int: bool,
	/// Tells whether interrupts were enabled before locking. This field is used only if `int` is
	/// `true`.
	int_enabled: bool,

	/// The data associated to the mutex.
	data: T,
}

/// Structure representing a Mutex.
pub struct Mutex<T: ?Sized> {
	/// An unsafe cell to the inner structure of the Mutex.
	inner: UnsafeCell<MutexIn<T>>,
}

impl<T> Mutex<T> {
	/// Creates a new Mutex with the given data to be owned.
	pub const fn new(data: T) -> Self {
		Self {
			inner: UnsafeCell::new(MutexIn {
				spin: Spinlock::new(),
				int: false,
				int_enabled: false,

				data,
			})
		}
	}
}

impl<T: ?Sized> Mutex<T> {
	/// Tells whether the mutex is already locked. This function should not be called to check if
	/// the mutex is ready to be locked before locking it, since it may cause race conditions. In
	/// this case, prefer using `lock` directly.
	pub fn is_locked(&self) -> bool {
		unsafe { // Safe because using the spinlock
			(*self.inner.get()).spin.is_locked()
		}
	}

	/// Locks the mutex. If the mutex is already locked, the thread shall wait until it becomes
	/// available.
	/// If `interrupt` is false, interruptions are disabled while locked, then restored when
	/// unlocked.
	/// The function returns a MutexGuard associated with the Mutex.
	pub fn lock(&self, interrupt: bool) -> MutexGuard<T> {
		let inner = unsafe { // Safe because using the spinlock later
			&mut *self.inner.get()
		};

		if interrupt {
			inner.spin.lock();

			// Setting the value after locking to avoid writing on it whilst the mutex was locked
			inner.int = true;

			MutexGuard::new(self)
		} else {
			let int_enabled = idt::is_interrupt_enabled();

			// Here is assumed that no interruption will change eflags. Which could cause a race
			// condition

			// Disabling interrupts before locking to ensure no interrupt will occure while locking
			crate::cli!();
			inner.spin.lock();

			// Setting the values after locking to avoid writing on them whilst the mutex was
			// locked
			inner.int = false;
			inner.int_enabled = int_enabled;

			MutexGuard::new(self)
		}
	}

	/// Returns an immutable reference to the payload. This function is unsafe because it can
	/// return the payload while the Mutex isn't locked.
	pub unsafe fn get_payload(&self) -> &T {
		&(*self.inner.get()).data
	}

	/// Returns a mutable reference to the payload. This function is unsafe because it can return
	/// the payload while the Mutex isn't locked.
	pub unsafe fn get_mut_payload(&self) -> &mut T {
		&mut (*self.inner.get()).data
	}

	/// Unlocks the mutex. The function is unsafe because it may lead to concurrency issues if not
	/// used properly.
	pub unsafe fn unlock(&self) {
		let inner = &mut (*self.inner.get());

		let int = inner.int;
		let int_enabled = inner.int_enabled;

		inner.spin.unlock();

		if !int {
			// Restoring interrupts state after unlocking
			if int_enabled {
				crate::sti!();
			} else {
				crate::cli!();
			}
		}
	}
}

unsafe impl<T> Sync for Mutex<T> {}

impl<T: ?Sized> Drop for Mutex<T> {
	fn drop(&mut self) {
		if self.is_locked() {
			panic!("Dropping a locked mutex");
		}
	}
}
