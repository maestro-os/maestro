/// This module contains the Mutex and MutexGuard structure.

use crate::util::lock::spinlock::Spinlock;

/// This structure is used to give access to a payload owned by a concurrency control structure.
pub struct LockPayload<'a, T> {
	/// A mutable reference to the Mutex.
	mutex: &'a mut Mutex::<T>,
}

impl<'a, T> LockPayload<'a, T> {
	/// Creates a new lock payload instance.
	pub fn new(mutex: &'a mut Mutex::<T>) -> Self {
		Self {
			mutex: mutex,
		}
	}

	/// Gives access to the payload.
	pub fn get(&self) -> &T {
		&self.mutex.data
	}

	/// Gives access to the payload.
	pub fn get_mut(&mut self) -> &mut T {
		&mut self.mutex.data
	}
}

impl<'a, T> Drop for LockPayload<'a, T> {
	/// Called when the LockPayload gets out of the scope of execution.
	fn drop(&mut self) {
		unsafe { // Call to unsafe function
			self.mutex.unlock();
		}
	}
}

/// Mutual exclusion for protection of sensitive data.
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

	/// Locks the mutex. If the mutex is already locked, the thread shall wait until it becomes
	/// available.
	pub fn lock(&mut self) -> LockPayload<T> {
		self.spin.lock();
		LockPayload::<T>::new(self)
	}

	/// Unlocks the Mutex.
	unsafe fn unlock(&mut self) {
		self.spin.unlock();
	}

	/// Returns an immutable reference to the payload. This function is unsafe because it can return
	/// the payload while the Mutex isn't locked.
	unsafe fn get_payload(&self) -> &T {
		&self.data
	}

	/// Returns a mutable reference to the payload. This function is unsafe because it can return
	/// the payload while the Mutex isn't locked.
	unsafe fn get_mut_payload(&mut self) -> &mut T {
		&mut self.data
	}
}

// TODO Remove MutexGuard?

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
		g.mutex.lock();
		g
	}

	/// Returns an immutable reference to the data owned by the associated Mutex.
	pub fn get(&self) -> &T {
		unsafe { // Call to unsafe function
			self.mutex.get_payload()
		}
	}

	/// Returns a mutable reference to the data owned by the associated Mutex.
	pub fn get_mut(&mut self) -> &mut T {
		unsafe { // Call to unsafe function
			self.mutex.get_mut_payload()
		}
	}
}

impl<'a, T> Drop for MutexGuard<'a, T> {
	/// Called when the MutexGuard gets out of the scope of execution.
	fn drop(&mut self) {
		unsafe { // Call to unsafe function
			self.mutex.unlock();
		}
	}
}
