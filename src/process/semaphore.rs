//! This module contains the Semaphore structure.

use crate::errno::Errno;
use crate::util::container::vec::Vec;
use crate::util::lock::*;
use super::Pid;

/// A semaphore is a structure which locks access to a data such that only one thread can access it
/// at the same time, and in the same order at they tried to acquire it (meaning that the threads
/// are handled in a FIFO fashion).
pub struct Semaphore<T> {
	/// The data wrapped by the semaphore.
	data: T,

	/// The FIFO containing the processes waiting to acquire the resource.
	fifo: Mutex<Vec<Pid>>, // TODO Use a dedicated FIFO structure
}

impl<T> Semaphore<T> {
	/// Creates a new semaphore with the given data `data`.
	pub fn new(data: T) -> Self {
		Self {
			data,

			fifo: Mutex::new(Vec::new()),
		}
	}

	/// Tells whether the process with PID `pid` can acquire the resource.
	fn can_acquire(&mut self, pid: Pid) -> bool {
		let guard = self.fifo.lock();
		let fifo = guard.get();
		fifo.is_empty() || fifo[0] == pid
	}

	/// Tries to acquire the object wrapped by the semaphore. If the object is already in use,
	/// the current process is set to `Sleeping` state.
	/// If the process dies while using the resource, it shall be removed automaticaly from the
	/// semaphore and the resource shall be made available for the next process.
	/// If this function is called while no process is running, the behaviour is undefined.
	/// `pid` is the PID of the process using the resource.
	pub fn acquire<F: Fn(&mut T)>(&mut self, pid: Pid, f: F) -> Result<(), Errno> {
		self.fifo.lock().get_mut().push(pid)?;

		while !self.can_acquire(pid) {
			crate::wait();
		}

		f(&mut self.data);

		{
			let mut guard = self.fifo.lock();
			debug_assert!(!guard.get_mut().is_empty());
			guard.get_mut().remove(0);
		}
		Ok(())
	}
}

impl<T> Drop for Semaphore<T> {
	fn drop(&mut self) {
		// TODO Return an errno for every waiting processes?
		todo!();
	}
}
