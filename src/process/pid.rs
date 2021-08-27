//! This module handles process PIDs.
//! Each process must have an unique PID, thus they have to be allocated. The kernel uses a
//! bitfield to store the used PIDs.

use crate::errno::Errno;
use crate::util::container::id_allocator::IDAllocator;

/// Type representing a Process ID. This ID is unique for every running processes.
pub type Pid = u16;

/// The maximum possible PID.
const MAX_PID: Pid = 32768;
/// The PID of the init process.
pub const INIT_PID: Pid = 1;

/// A structure handling PID allocations.
pub struct PIDManager {
	/// The PID allocator.
	allocator: IDAllocator,
}

impl PIDManager {
	/// Creates a new instance.
	pub fn new() -> Result<Self, Errno> {
		let mut s = Self {
			allocator: IDAllocator::new(MAX_PID as _)?,
		};
		s.allocator.set_used((INIT_PID - 1) as _);
		Ok(s)
	}

	/// Returns a unused PID and marks it as used.
	pub fn get_unique_pid(&mut self) -> Result<Pid, Errno> {
		match self.allocator.alloc(None) {
			Ok(i) => {
				debug_assert!(i <= MAX_PID as _);

				Ok((i + 1) as _)
			},
			Err(e) => {
				Err(e)
			}
		}
	}

	/// Releases the given PID `pid` to make it available for other processes.
	/// If the PID wasn't allocated, the function does nothing.
	pub fn release_pid(&mut self, pid: Pid) {
		debug_assert!(pid >= 1);
		debug_assert!(pid <= MAX_PID as _);

		self.allocator.free((pid - 1) as _)
	}
}
