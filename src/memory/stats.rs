//! Statistics about memory usage.

use crate::util::lock::Mutex;
use core::fmt;
use core::fmt::{Display, Formatter};

/// Stores memory usage information. Each field is in KiB.
pub struct MemInfo {
	/// The total amount of memory on the system.
	pub mem_total: usize,
	/// The total amount of free physical memory.
	pub mem_free: usize,
}

impl Display for MemInfo {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		writeln!(
			f,
			"MemTotal: {} kB
MemFree: {} kB",
			self.mem_total, self.mem_free,
		)
	}
}

/// Memory usage statistics.
pub static MEM_INFO: Mutex<MemInfo> = Mutex::new(MemInfo {
	mem_total: 0,
	mem_free: 0,
});
