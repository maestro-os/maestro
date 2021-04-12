/// This module implements the scope end hook which allows to perform an action when the control
/// flow goes out of the scope it was allocated in.

/// TODO doc
pub struct ScopeEndHook<T: Fn()> {
	/// A closure to execute when the structure is dropped.
	f: T,
}

impl<T: Fn()> Drop for ScopeEndHook::<T> {
	fn drop(&mut self) {
		(self.f)();
	}
}
