//! The Base Address Register (BAR) is a way to communicate with a device using Direct Access
//! Memory (DMA).

/// Enumeration of Memory Space BAR types.
#[derive(Clone, Debug)]
pub enum BARType {
	/// The register is 32 bits wide.
	Size32,
	/// The register is 64 bits wide.
	Size64,
}

/// Structure representing a Base Address Register.
#[derive(Clone, Debug)]
pub enum BAR {
	MemorySpaceBAR {
		/// The type of the BAR, specifying the size of the register.
		type_: BARType,
		/// If true, read accesses don't have any side effects.
		prefetchable: bool,

		/// Physical address to the register.
		address: u64,

		/// The size of the address space in bytes.
		size: usize,
	},

	IOSpaceBAR {
		/// Physical address to the register.
		address: u64,

		/// The size of the address space in bytes.
		size: usize,
	},
}

impl BAR {
	/// Returns the base address.
	pub fn get_physical_address(&self) -> Option<*mut ()> {
		let (addr, size) = match self {
			Self::MemorySpaceBAR { address, size, .. } => (*address, *size),
			Self::IOSpaceBAR { address, size, .. } => (*address, *size),
		};

		if (addr + size as u64) > usize::MAX as u64 {
			Some(addr as _)
		} else {
			None
		}
	}

	/// Returns the amount of memory.
	pub fn get_size(&self) -> usize {
		match self {
			Self::MemorySpaceBAR { size, .. } => *size,
			Self::IOSpaceBAR { size, .. } => *size,
		}
	}

	/// Tells whether the memory is prefetchable.
	pub fn is_prefetchable(&self) -> bool {
		match self {
			Self::MemorySpaceBAR { prefetchable, .. } => *prefetchable,
			Self::IOSpaceBAR { .. } => false,
		}
	}
}
