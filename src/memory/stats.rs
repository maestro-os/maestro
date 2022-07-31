//! This module implements statistics about memory usage.

use crate::errno::Errno;
use crate::util::container::string::String;
use crate::util::lock::Mutex;

/// This structure stores memory usage informations. Each field is in KiB.
pub struct MemInfo {
	/// The total amount of memory on the system.
	pub mem_total: usize,
	/// The total amount of free physical memory.
	pub mem_free: usize,
	// TODO
}

impl MemInfo {
	/// Returns the string representation of the current structure.
	pub fn to_string(&self) -> Result<String, Errno> {
		crate::format!(
			"MemTotal: {} kB
MemFree: {} kB
",
			self.mem_total,
			self.mem_free,
		)
	}
}

/// The global variable storing memory usage informations.
pub static MEM_INFO: Mutex<MemInfo> = Mutex::new(MemInfo {
	mem_total: 0,
	mem_free: 0,
	// TODO
});
