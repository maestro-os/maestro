//! This module handles minor/major numbers, including their allocation.

use crate::{
	device::DeviceType,
	errno::AllocResult,
	util::{container::id_allocator::IDAllocator, lock::Mutex},
};
use core::cell::OnceCell;

/// The number of major numbers.
const MAJOR_COUNT: u32 = 256;
/// The number of minor numbers.
const MINORS_COUNT: u32 = 256;

/// Returns the major number from a device number.
pub fn major(dev: u64) -> u32 {
	(((dev >> 8) & 0xfff) | ((dev >> 32) & !0xfff)) as _
}

/// Returns the minor number from a device number.
pub fn minor(dev: u64) -> u32 {
	((dev & 0xff) | ((dev >> 12) & !0xff)) as _
}

/// Returns a device number from a major/minor pair.
pub const fn makedev(major: u32, minor: u32) -> u64 {
	(((minor & 0xff) as u64)
		| (((major & 0xfff) as u64) << 8)
		| (((minor & !0xff) as u64) << 12)
		| (((major & !0xfff) as u64) << 32)) as _
}

/// A block of minor numbers associated with a unique major number, and it can allocate every minor
/// numbers with it.
pub struct MajorBlock {
	/// The device type.
	device_type: DeviceType,
	/// The block's major number.
	major: u32,

	/// The minor number allocator.
	allocator: IDAllocator,
}

impl MajorBlock {
	/// Creates a new instance with the given major number `major`.
	fn new(device_type: DeviceType, major: u32) -> AllocResult<Self> {
		Ok(Self {
			device_type,
			major,

			allocator: IDAllocator::new(MINORS_COUNT)?,
		})
	}

	/// Returns the device type.
	pub fn get_device_type(&self) -> DeviceType {
		self.device_type
	}

	/// Returns the major number associated with the block.
	pub fn get_major(&self) -> u32 {
		self.major
	}

	/// Allocates a minor number on the current major number block.
	///
	/// If `minor` is not `None`, the function shall allocate the given minor
	/// number.
	///
	/// If the allocation fails, the function returns an `Err`.
	pub fn alloc_minor(&mut self, minor: Option<u32>) -> AllocResult<u32> {
		self.allocator.alloc(minor)
	}

	/// Frees the given minor number in the current block.
	pub fn free_minor(&mut self, minor: u32) {
		self.allocator.free(minor);
	}
}

impl Drop for MajorBlock {
	fn drop(&mut self) {
		let mut major_allocator = match self.device_type {
			DeviceType::Block => BLOCK_MAJOR_ALLOCATOR.lock(),
			DeviceType::Char => CHAR_MAJOR_ALLOCATOR.lock(),
		};
		if let Some(major_allocator) = major_allocator.get_mut() {
			major_allocator.free(self.major);
		}
	}
}

/// The major numbers allocator.
static BLOCK_MAJOR_ALLOCATOR: Mutex<OnceCell<IDAllocator>> = Mutex::new(OnceCell::new());
/// The major numbers allocator.
static CHAR_MAJOR_ALLOCATOR: Mutex<OnceCell<IDAllocator>> = Mutex::new(OnceCell::new());

/// Allocates a major number.
///
/// `device_type` is the type of device for the major block to be allocated.
///
/// If `major` is not `None`, the function shall allocate the specific given major
/// number.
///
/// If the allocation fails, the function returns an `Err`.
pub fn alloc_major(device_type: DeviceType, major: Option<u32>) -> AllocResult<MajorBlock> {
	let mut major_allocator = match device_type {
		DeviceType::Block => BLOCK_MAJOR_ALLOCATOR.lock(),
		DeviceType::Char => CHAR_MAJOR_ALLOCATOR.lock(),
	};
	major_allocator.get_or_try_init(|| IDAllocator::new(MAJOR_COUNT))?;
	// FIXME: remove unwrap (wait until `get_mut_or_try_init` or equivalent is available)
	let major_allocator = major_allocator.get_mut().unwrap();
	let major = major_allocator.alloc(major)?;
	MajorBlock::new(device_type, major)
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn device_number() {
		for maj in 0..256 {
			for min in 0..MINORS_COUNT {
				let dev = makedev(maj, min);
				assert_eq!(major(dev), maj);
				assert_eq!(minor(dev), min);
			}
		}
	}
}
