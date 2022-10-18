//! This module implements locks, useful to prevent race conditions in multithreaded code for
//! example.
//!
//! Mutual exclusion is used to protect data from concurrent access.
//! A Mutex allows to ensure that one, and only thread accesses the data stored into it at the same
//! time. Preventing race conditions. They usually work using spinlocks.
//!
//! One particularity with kernel development is that multi-threading is not the only way to get
//! concurrency issues. Another factor to take into account is that fact that an interruption may
//! be triggered at any moment while executing the code unless disabled. For this reason, mutexes
//! in the kernel are equiped with an option allowing to disable interrupts while being locked.
//!
//! If an exception is raised while a mutex that disables interruptions is acquired, the behaviour
//! is undefined.

pub mod spinlock;

use core::cell::UnsafeCell;
use crate::idt;
use crate::util::lock::spinlock::Spinlock;

/// Structure representing the saved state of interruptions for the current thread.
struct State {
	/// The number of currently locked mutexes that disable interruptions.
	ref_count: usize,

	/// Tells whether interruptions were enabled before locking mutexes.
	enabled: bool,
}

// TODO When implementing multicore, use an array. One element per core
/// Saved state of interruptions for the current thread.
/// This variable doesn't require synchonization since interruptions are always disabled when it is
/// accessed.
static mut INT_DISABLE_REFS: State = State {
	ref_count: 0,

	enabled: false,
};

/// Type used to declare a guard meant to unlock the associated Mutex at the moment the execution
/// gets out of the scope of its declaration. This structure is useful to ensure that the mutex
/// doesen't stay locked after the exectution of a function ended.
pub struct MutexGuard<'a, T: ?Sized, const INT: bool> {
	/// The mutex associated to the guard
	mutex: &'a Mutex<T, INT>,
}

impl<'a, T: ?Sized, const INT: bool> MutexGuard<'a, T, INT> {
	/// Creates an instance of MutexGuard for the given mutex `mutex`.
	fn new(mutex: &'a Mutex<T, INT>) -> Self {
		Self { mutex }
	}

	/// Returns the mutex associated with the current guard.
	pub fn get_mutex(&self) -> &'a Mutex<T, INT> {
		self.mutex
	}

	/// Returns an immutable reference to the data owned by the associated Mutex.
	pub fn get(&self) -> &T {
		unsafe { self.mutex.get_payload() }
	}

	/// Returns a mutable reference to the data owned by the associated Mutex.
	pub fn get_mut(&self) -> &mut T {
		unsafe { self.mutex.get_mut_payload() }
	}

	/// Unlocks the Mutex.
	pub fn unlock(self) {}
}

impl<'a, T: ?Sized, const INT: bool> Drop for MutexGuard<'a, T, INT> {
	fn drop(&mut self) {
		unsafe {
			self.mutex.unlock();
		}
	}
}

/// The inner structure of the Mutex structure.
struct MutexIn<T: ?Sized, const INT: bool> {
	/// The spinlock for the underlying data.
	spin: Spinlock,

	/// Saved callstack, used to debug deadlocks.
	#[cfg(config_debug_deadlock_stack)]
	saved_stack: [*mut core::ffi::c_void; 32],

	/// The data associated to the mutex.
	data: T,
}

/// Structure representing a Mutex.
/// The object wrapped in this structure can be accessed by only one thread at a time.
/// The `INT` generic parameter tells whether interrupts are allowed while the mutex is locked. The
/// default value is `true`.
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

				#[cfg(config_debug_deadlock_stack)]
				saved_stack: [core::ptr::null_mut::<core::ffi::c_void>(); 32],
			}),
		}
	}
}

impl<T: ?Sized, const INT: bool> Mutex<T, INT> {
	/// Tells whether the mutex is already locked. This function should not be called to check if
	/// the mutex is ready to be locked before locking it, since it may cause race conditions. In
	/// this case, prefer using `lock` directly.
	pub fn is_locked(&self) -> bool {
		unsafe {
			// Safe because using the spinlock
			(*self.inner.get()).spin.is_locked()
		}
	}

	/// Locks the mutex. If the mutex is already locked, the thread shall wait until it becomes
	/// available.
	/// The function returns a MutexGuard associated with the Mutex.
	pub fn lock(&self) -> MutexGuard<T, INT> {
		let inner = unsafe {
			// Safe because using the spinlock later
			&mut *self.inner.get()
		};

		let state = idt::is_interrupt_enabled();

		// Here is assumed that no interruption will change eflags' INT. Which could cause a race
		// condition

		if !INT {
			// Disabling interrupts before locking to ensure no interrupt will occure while locking
			crate::cli!();
		}

		inner.spin.lock();

		if !INT {
			// Updating the current thread's state
			// Safe because interrupts are disabled and the value can be accessed only by the
			// current core
			unsafe {
				if INT_DISABLE_REFS.ref_count == 0 {
					INT_DISABLE_REFS.enabled = state;
				}
				INT_DISABLE_REFS.ref_count += 1;
			}
		}

		// If enabled, save the stack to debug deadlocks
		#[cfg(config_debug_deadlock_stack)]
		{
			let ebp = unsafe { crate::register_get!("ebp") as *mut _ };
			crate::debug::get_callstack(ebp, &mut inner.saved_stack);
		}

		MutexGuard::new(self)
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
	/// If the mutex is not locked, the behaviour is undefined.
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

impl<T: ?Sized, const INT: bool> Drop for Mutex<T, INT> {
	fn drop(&mut self) {
		// This condition should never be fulfilled without using `unsafe` since lifetimes ensure
		// the lock guard has to be dropped before the mutex
		if self.is_locked() {
			panic!("Dropping a locked mutex");
		}
	}
}

/// Type alias on Mutex representing a mutex which blocks interrupts.
pub type IntMutex<T> = Mutex<T, false>;
