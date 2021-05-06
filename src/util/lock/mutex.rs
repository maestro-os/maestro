/// This module contains the Mutex and MutexGuard structure.

use crate::util::lock::spinlock::Spinlock;

/// Type used to declare a guard meant to unlock the associated Mutex at the moment the execution
/// gets out of the scope of its declaration. This structure is useful to ensure that the mutex
/// doesen't stay locked after the exectution of a function ended.
pub struct MutexGuard<'a, T> {
	/// The mutex associated to the guard
	mutex: &'a mut Mutex<T>,
}

impl<'a, T> MutexGuard<'a, T> {
	/// Creates an instance of MutexGuard for the given `mutex` and locks it.
	pub fn new(mutex: &'a mut Mutex<T>) -> Self {
		let g = Self {
			mutex: mutex,
		};
		g.mutex.spin.lock();
		g
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

impl<'a, T> Drop for MutexGuard<'a, T> {
	/// Called when the MutexGuard gets out of the scope of execution.
	fn drop(&mut self) {
		self.mutex.spin.unlock();
	}
}

/// Mutual exclusion, used to protect data from concurrent access.
/// A Mutex allows to ensure that one, and only thread accesses the data stored into it at the same
/// time. Preventing race conditions.
/// This structure works using spinlocks.
pub struct Mutex<T> {
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
			data: data,
		}
	}

	/// Tells whether the mutex is already locked. This function should not be called to check if
	/// the mutex is ready to be locked before locking it, since it may cause race conditions. In
	/// this case, prefer using `lock` directly.
	pub fn is_locked(&self) -> bool {
		self.spin.is_locked()
	}

	/// Locks the mutex. If the mutex is already locked, the thread shall wait until it becomes
	/// available.
	/// The function returns a MutexGuard associated with the Mutex.
	pub fn lock(&mut self) -> MutexGuard<T> {
		MutexGuard::new(self)
	}

	/// Returns an immutable reference to the payload. This function is unsafe because it can return
	/// the payload while the Mutex isn't locked.
	pub unsafe fn get_payload(&self) -> &T {
		&self.data
	}

	/// Returns a mutable reference to the payload. This function is unsafe because it can return
	/// the payload while the Mutex isn't locked.
	pub unsafe fn get_mut_payload(&mut self) -> &mut T {
		&mut self.data
	}
}
