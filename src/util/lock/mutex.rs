//! This module contains the Mutex and MutexGuard structure.
//! 
//! Mutual exclusion is used to protect data from concurrent access.
//! A Mutex allows to ensure that one, and only thread accesses the data stored into it at the same
//! time. Preventing race conditions.
//!
//! A Mutex usually works using spinlocks.

use core::marker::PhantomData;
use crate::idt;
use crate::util::lock::spinlock::Spinlock;

/// Trait representing a Mutex.
pub trait TMutex<T: ?Sized> {
	/// Tells whether the mutex is already locked. This function should not be called to check if
	/// the mutex is ready to be locked before locking it, since it may cause race conditions. In
	/// this case, prefer using `lock` directly.
	fn is_locked(&self) -> bool;
	/// Locks the mutex. If the mutex is already locked, the thread shall wait until it becomes
	/// available.
	/// The function returns a MutexGuard associated with the Mutex.
	fn lock(&mut self) -> MutexGuard<T, Self>;

	/// Returns an immutable reference to the payload. This function is unsafe because it can return
	/// the payload while the Mutex isn't locked.
	unsafe fn get_payload(&self) -> &T;
	/// Returns a mutable reference to the payload. This function is unsafe because it can return
	/// the payload while the Mutex isn't locked.
	unsafe fn get_mut_payload(&mut self) -> &mut T;
	/// Unlocks the mutex. The function is unsafe because it may lead to concurrency issues if not
	/// used properly.
	unsafe fn unlock(&mut self);
}

/// Type used to declare a guard meant to unlock the associated Mutex at the moment the execution
/// gets out of the scope of its declaration. This structure is useful to ensure that the mutex
/// doesen't stay locked after the exectution of a function ended.
pub struct MutexGuard<'a, T: ?Sized, M: TMutex<T> + ?Sized> {
	/// The mutex associated to the guard
	mutex: &'a mut M,

	_data: PhantomData<T>,
}

impl<'a, T: ?Sized, M: TMutex<T> + ?Sized> MutexGuard<'a, T, M> {
	/// Creates an instance of MutexGuard for the given `mutex` and locks it.
	pub fn new(mutex: &'a mut M) -> Self {
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

impl<'a, T: ?Sized, M: TMutex<T> + ?Sized> Drop for MutexGuard<'a, T, M> {
	/// Called when the MutexGuard gets out of the scope of execution.
	fn drop(&mut self) {
		unsafe {
			self.mutex.unlock();
		}
	}
}

/// Structure representing a Mutex.
pub struct Mutex<T: ?Sized> {
	/// The spinlock for the underlying data.
	spin: Spinlock,
	/// The data associated to the mutex.
	data: T,
}

impl<T> Mutex<T> {
	/// Creates a new Mutex with the given data to be owned.
	pub const fn new(data: T) -> Self {
		Self {
			spin: Spinlock::new(),
			data,
		}
	}
}

impl<T: ?Sized> TMutex<T> for Mutex<T> {
	fn is_locked(&self) -> bool {
		self.spin.is_locked()
	}

	fn lock(&mut self) -> MutexGuard<T, Self> {
		self.spin.lock();
		MutexGuard::new(self)
	}

	unsafe fn get_payload(&self) -> &T {
		&self.data
	}

	unsafe fn get_mut_payload(&mut self) -> &mut T {
		&mut self.data
	}

	unsafe fn unlock(&mut self) {
		self.spin.unlock();
	}
}

unsafe impl<T> Sync for Mutex<T> {}

/// Structure representing an Mutex which disables interruptions while the object is locked.
pub struct InterruptMutex<T: ?Sized> {
	/// The spinlock for the underlying data.
	spin: Spinlock,
	/// Tells whether interruptions were enabled before locking.
	interrupt_enabled: bool,

	/// The data associated to the mutex.
	data: T,
}

impl<T> InterruptMutex<T> {
	/// Creates a new instance with the given data to be owned.
	pub const fn new(data: T) -> Self {
		Self {
			spin: Spinlock::new(),
			interrupt_enabled: false,

			data,
		}
	}
}

impl<T: ?Sized> TMutex<T> for InterruptMutex<T> {
	fn is_locked(&self) -> bool {
		self.spin.is_locked()
	}

	fn lock(&mut self) -> MutexGuard<T, Self> {
		self.interrupt_enabled = idt::is_interrupt_enabled();
		crate::cli!();
		self.spin.lock();

		MutexGuard::new(self)
	}

	unsafe fn get_payload(&self) -> &T {
		&self.data
	}

	unsafe fn get_mut_payload(&mut self) -> &mut T {
		&mut self.data
	}

	unsafe fn unlock(&mut self) {
		if self.is_locked() {
			self.spin.unlock();
			if self.interrupt_enabled {
				crate::sti!();
			}
		}
	}
}

unsafe impl<T: ?Sized> Sync for InterruptMutex<T> {}
