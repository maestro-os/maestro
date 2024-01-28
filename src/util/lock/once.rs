//! Once-initialized objects.

use core::{cell::UnsafeCell, mem::MaybeUninit};

/// An object that is meant to be initialized once at boot, then accessed in read-only.
///
/// The value **must** be initialized with `init` before calling `get`. Failure to do so results in
/// an undefined behavior.
pub struct OnceInit<T> {
	/// The inner value. If `None`, it has not been initialized yet.
	val: UnsafeCell<MaybeUninit<T>>,
}

impl<T> OnceInit<T> {
	/// Creates a new instance waiting to be initialized.
	///
	/// # Safety
	///
	/// The value **must** be initialized with before calling `get`.
	pub const unsafe fn new() -> Self {
		Self {
			val: UnsafeCell::new(MaybeUninit::uninit()),
		}
	}

	/// Initializes with the given value.
	///
	/// If already initialized, the previous value is **not** dropped.
	///
	/// # Safety
	///
	/// It is the caller's responsibility to enforce concurrency rules.
	pub unsafe fn init(&self, val: T) {
		(*self.val.get()).write(val);
	}

	/// Returns the inner value.
	pub fn get(&self) -> &T {
		unsafe { (*self.val.get()).assume_init_ref() }
	}
}

unsafe impl<T> Sync for OnceInit<T> {}
