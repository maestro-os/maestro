/// This module implements utility for current code, useful to prevent race conditions for example.

/// Extern spinlock functions.
extern "C" {
	pub fn spin_lock(lock: *mut i32);
	pub fn spin_unlock(lock: *mut i32);
}

/// Structure representing a spinlock.
pub struct Spinlock {
	/// Variable telling whether the spinlock is locked or not. This variable is 4 bytes wide to
	/// match the size of the register handling it.
	locked: i32,
}

impl Spinlock {
	/// Creates a new spinlock.
	pub const fn new() -> Self {
		Self {
			locked: 0,
		}
	}

	/// Wrapper for `spin_lock`. Locks the spinlock.
	pub fn lock(&mut self) {
		unsafe {
			spin_lock(&mut self.locked);
		}
	}

	/// Wrapper for `spin_unlock`. Unlocks the spinlock.
	pub fn unlock(&mut self) {
		unsafe {
			spin_unlock(&mut self.locked);
		}
	}
}

/// This structure is used to give access to a payload owned by a concurrency control structure.
pub struct LockPayload<'a, T> {
	/// A mutable reference to the payload owned by the sturcture 
	payload: &'a mut T,
}

impl<'a, T> LockPayload<'a, T> {
	/// Creates a new lock payload instance.
	pub fn new(payload: &'a mut T) -> Self {
		Self {
			payload: payload,
		}
	}

	/// Gives access to the payload.
	pub fn get_mut(&mut self) -> &mut T {
		&mut self.payload
	}
}

/// Mutual exclusion for protection of sensitive data.
/// A Mutex allows to ensure that one, and only thread accesses the data stored into it at the same
/// time. Preventing race conditions.
pub struct Mutex<T> {
	spin: Spinlock,
	data: T,
}

impl<T> Mutex<T> {
	/// Creates a new Mutex with the given data to be owned.
	pub fn new(data: T) -> Self {
		Self {
			spin: Spinlock::new(),
			data: data,
		}
	}

	/// Locks the mutex. If the mutex is already locked, the thread shall wait until it becomes
	/// available.
	pub fn lock(&mut self) -> LockPayload<T> {
		self.spin.lock();
		LockPayload::<T>::new(&mut self.data)
	}

	// TODO Protect against unlocking while the payload is still in use
	/// Unlocks the mutex. Does nothing if the mutex is already unlocked.
	pub fn unlock(&mut self) {
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

impl<'a, T> core::ops::Drop for MutexGuard<'a, T> {
	/// Called when the MutexGuard gets out of the scope of execution.
	fn drop(&mut self) {
		self.mutex.unlock();
	}
}

// TODO Semaphore
