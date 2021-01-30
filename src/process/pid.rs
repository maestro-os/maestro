/// This module handles process PIDs.
/// Each process must have an unique PID, thus they have to be allocated. The kernel uses a bitfield to store the used
/// PIDs.

use crate::util::container::bitfield::Bitfield;

/// Type representing a Process ID. This ID is unique for every running processes.
pub type Pid = u16;

/// The maximum possible PID.
const MAX_PID: Pid = 32768;

/// A structure handling PID allocations.
pub struct PIDManager {
	/// The bitfield storing which PIDs are allocated.
	used: Bitfield,
	/// The cursor, indicating which PID to check next in the bitfield.
	cursor: usize,
}

impl PIDManager {
	/// Creates a new instance.
	pub fn new() -> Result::<Self, ()> {
		Ok(Self {
			used: Bitfield::new((MAX_PID + 1) as _)?,
			cursor: 0,
		})
	}

	/// Increments the cursor.
	fn increment_cursor(&mut self) {
		self.cursor = (self.cursor + 1) % self.used.len();
	}

	/// Returns a unused PID and marks it as used.
	pub fn get_unique_pid(&mut self) -> Result::<Pid, ()> {
		if self.used.set_count() >= self.used.len() {
			return Err(());
		}

		while self.used.is_set(self.cursor) {
			self.increment_cursor();
		}

		let pid = self.cursor;
		self.used.set(pid);
		self.increment_cursor();

		Ok(pid as _)
	}

	/// Releases the given PID `pid` to make it available for other processes.
	pub fn release_pid(&mut self, pid: Pid) {
		self.used.clear(pid as _);
	}
}
