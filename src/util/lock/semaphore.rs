/// This module contains the Semaphore structure.

/// A semaphore is a structure which locks access to a data such that only one thread can access it
/// at the same time, and in the same order at they tried to acquire it (meaning that the threads
/// are handled in a FIFO fashion).
pub struct Semaphore<T> {
	/// The data wrapped by the semaphore.
	data: T,
	// TODO Threads FIFO
}

impl<T> Semaphore<T> {
	/// Creates a new semaphore with the given data `data`.
	pub fn new(data: T) -> Self {
		Self {
			data: data,
			// TODO
		}
	}

	// TODO
}
