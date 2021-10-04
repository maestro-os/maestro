//! The Base Address Register (BAR) is a way to communicate with a device using Direct Access
//! Memory (DMA).

use crate::device::bus::pci::PCIDevice;

/// Structure representing a Base Address Register.
pub struct BAR {
	/// The base address.
	address: u64,
	/// The amount of space required by the device.
	size: usize,

	/// The BAR type.
	type_: u8,
	/// Tells whether the memory is prefetchable.
	prefetchable: bool,
}

impl BAR {
	/// Creates a new instance from a PCI device.
	/// `dev` is the PCI device.
	/// `n` is the BAR id.
	/// If the BAR doesn't exist, the function returns None.
	pub fn from_pci(dev: &PCIDevice, n: u8) -> Option<Self> {
		// The BAR's value
		let value = dev.get_bar_value(n)?;

		// TODO Get size
		let size = 0;

		if (value & 0b1) == 0 {
			let type_ = ((value >> 1) & 0b11) as u8;
			let address = match type_ {
				0x0 => (value & 0xfffffff0) as u64,
				0x1 => (value & 0xfff0) as u64,
				0x2 => {
					// The next BAR's value
					let next_value = dev.get_bar_value(n + 1)?;
					(value & 0xfffffff0) as u64 | ((next_value as u64) << 32)
				},

				_ => 0,
			};

			Some(Self {
				address,
				size,

				type_,
				prefetchable: value & 0b1000 != 0,
			})
		} else {
			Some(Self {
				address: (value & 0xfffffffc) as u64,
				size,

				type_: 0,
				prefetchable: false,
			})
		}
	}

	/// Returns the base address.
	#[inline(always)]
	pub fn get_physical_address(&self) -> u64 {
		self.address
	}

	/// Returns the amount of memory.
	#[inline(always)]
	pub fn get_size(&self) -> usize {
		self.size
	}

	/// Returns the type of the BAR.
	#[inline(always)]
	pub fn get_type(&self) -> u8 {
		self.type_
	}

	/// Tells whether the memory is prefetchable.
	#[inline(always)]
	pub fn is_prefetchable(&self) -> bool {
		self.prefetchable
	}
}
