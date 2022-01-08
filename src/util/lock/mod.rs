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
use core::mem::MaybeUninit;
use crate::idt;
use crate::util::lock::spinlock::Spinlock;

// TODO Add a deadlock detection mechanism (and mention it into the documentation)

/// Type used to declare a guard meant to unlock the associated Mutex at the moment the execution
/// gets out of the scope of its declaration. This structure is useful to ensure that the mutex
/// doesen't stay locked after the exectution of a function ended.
pub struct MutexGuard<'a, T: ?Sized, I: IntManager> {
	/// The mutex associated to the guard
	mutex: &'a Mutex<T, I>,
}

impl<'a, T: ?Sized, I: IntManager> MutexGuard<'a, T, I> {
	/// Creates an instance of MutexGuard for the given mutex `mutex`.
	fn new(mutex: &'a Mutex<T, I>) -> Self {
		Self {
			mutex,
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

impl<'a, T: ?Sized, I: IntManager> Drop for MutexGuard<'a, T, I> {
	fn drop(&mut self) {
		unsafe {
			self.mutex.unlock();
		}
	}
}

/// Trait representing an interrupt manager.
/// When locking a resource, it way require the kernel to mask interrupts. This trait represents an
/// object which masks interrupts when locked.
/// This trait is meant to be used internally only.
pub trait IntManager {
	/// Returns the saved interrupt state.
	fn get_state(&self) -> bool;
	/// Saves the given interrupt state.
	fn set_state(&mut self, state: bool);
}

/// A dummy interrupt manager which doesn't do anything. Used by resources that do not need
/// interrupts to be masked.
/// This structure is meant to be used internally only.
pub struct DummyIntManager {}

impl IntManager for DummyIntManager {
	fn get_state(&self) -> bool {
		idt::is_interrupt_enabled()
	}

	fn set_state(&mut self, _: bool) {}
}

/// A normal interrupt manager.
/// This structure is meant to be used internally only.
pub struct NormalIntManager {
	/// Tells whether interrupts were enabled before locking.
	int_enabled: bool,
}

impl IntManager for NormalIntManager {
	fn get_state(&self) -> bool {
		self.int_enabled
	}

	fn set_state(&mut self, state: bool) {
		self.int_enabled = state;
	}
}

/// The inner structure of the Mutex structure.
struct MutexIn<T: ?Sized, I: IntManager> {
	/// The spinlock for the underlying data.
	spin: Spinlock,
	/// The interrupt manager.
	int_manager: MaybeUninit<I>,

	/// The data associated to the mutex.
	data: T,
}

/// Structure representing a Mutex.
/// The object wrapped in this structure can be accessed by only one thread at a time.
/// If interrupts need to be disabled while accessing, the type `IntMutex` can be used instead.
pub struct Mutex<T: ?Sized, I: IntManager = DummyIntManager> {
	/// An unsafe cell to the inner structure of the Mutex.
	inner: UnsafeCell<MutexIn<T, I>>,
}

impl<T, I: IntManager> Mutex<T, I> {
	/// Creates a new Mutex with the given data to be owned.
	pub const fn new(data: T) -> Self {
		Self {
			inner: UnsafeCell::new(MutexIn {
				spin: Spinlock::new(),
				int_manager: MaybeUninit::uninit(),

				data,
			})
		}
	}
}

impl<T: ?Sized, I: IntManager> Mutex<T, I> {
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
	/// The function returns a MutexGuard associated with the Mutex.
	pub fn lock(&self) -> MutexGuard<T, I> {
		let inner = unsafe { // Safe because using the spinlock later
			&mut *self.inner.get()
		};

		let state = idt::is_interrupt_enabled();

		// Here is assumed that no interruption will change eflags' INT. Which could cause a race
		// condition

		// Disabling interrupts before locking to ensure no interrupt will occure while locking
		crate::cli!();
		inner.spin.lock();

		// Setting the values after locking to avoid writing on them whilst the mutex was
		// locked
		unsafe {
			inner.int_manager.assume_init_mut().set_state(state);
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

		// The state to restore
		let state = inner.int_manager.assume_init_ref().get_state();

		inner.spin.unlock();

		// Restoring interrupts state after unlocking
		if state {
			crate::sti!();
		} else {
			crate::cli!();
		}
	}
}

unsafe impl<T, I: IntManager> Sync for Mutex<T, I> {}

impl<T: ?Sized, I: IntManager> Drop for Mutex<T, I> {
	fn drop(&mut self) {
		if self.is_locked() {
			panic!("Dropping a locked mutex");
		}
	}
}

/// This type represents a mutex that works just like a normal one. Unless when locked, interrupts
/// are disabled. The interrupt state is then restored when the mutex is unlocked.
pub type IntMutex<T> = Mutex<T, NormalIntManager>;
