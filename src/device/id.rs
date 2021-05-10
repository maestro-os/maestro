//! This module handles minor/major numbers, including their allocation.

use crate::errno::Errno;
use crate::util::container::id_allocator::IDAllocator;
use crate::util::lock::mutex::Mutex;
use crate::util::lock::mutex::MutexGuard;

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
pub fn makedev(major: u32, minor: u32) -> u64 {
	(((minor & 0xff) as u64)
		| (((major & 0xfff) as u64) << 8)
		| (((minor & !0xff) as u64) << 12)
		| (((major & !0xfff) as u64) << 32)) as _
}

/// Structure representing a block of minor numbers. The structure is associated with a unique
/// major number, and it can allocate every minor numbers with it.
pub struct MajorBlock {
	/// The block's major number.
	major: u32,
	/// The minor number allocator.
	allocator: IDAllocator,
}

impl MajorBlock {
	/// Creates a new instance with the given major number `major`.
	fn new(major: u32) -> Result<Self, Errno> {
		Ok(Self {
			major: major,
			allocator: IDAllocator::new(MINORS_COUNT)?,
		})
	}

	/// Returns the major number associated with the block.
	pub fn get_major(&self) -> u32 {
		self.major
	}

	/// Allocates a minor number on the current major number block.
	/// If `minor` is not None, the function shall allocate the given minor number.
	/// If the allocation fails, the function returns an Err.
	pub fn alloc_minor(&mut self, minor: Option<u32>) -> Result<u32, Errno> {
		self.allocator.alloc(minor)
	}

	/// Frees the given minor number in the current block.
	pub fn free_minor(&mut self, minor: u32) {
		self.allocator.free(minor);
	}
}

impl Drop for MajorBlock {
	fn drop(&mut self) {
		free_major(self);
	}
}

/// The major numbers allocator.
static mut MAJOR_ALLOCATOR: Mutex<Option<IDAllocator>> = Mutex::new(None);
// TODO
///// The list of major blocks allocated for dynamicaly allocated minor/major pairs.
//static mut DYN_MAJORS: Mutex<Vec<MajorBlock>> = Mutex::new(Vec::new());

/// Allocates a major number.
/// If `major` is not None, the function shall allocate the specific given major number.
/// If the allocation fails, the function returns an Err.
pub fn alloc_major(major: Option<u32>) -> Result<MajorBlock, Errno> {
	let mutex = unsafe { // Safe because using Mutex
		&mut MAJOR_ALLOCATOR
	};
	let mut guard = MutexGuard::new(mutex);
	let major_allocator = guard.get_mut();
	if major_allocator.is_none() {
		*major_allocator = Some(IDAllocator::new(MAJOR_COUNT)?);
	}
	let major_allocator = major_allocator.as_mut().unwrap();

	let major = major_allocator.alloc(major)?;
	let block = MajorBlock::new(major)?;
	Ok(block)
}

/// Frees the given major block `block`.
/// **WARNING**: This function should be called directly, but only from the MajorBlock itself.
pub fn free_major(block: &mut MajorBlock) {
	let mutex = unsafe { // Safe because using Mutex
		&mut MAJOR_ALLOCATOR
	};
	let mut guard = MutexGuard::new(mutex);
	let major_allocator = guard.get_mut().as_mut().unwrap();

	major_allocator.free(block.get_major());
}

/* TODO
/// Allocates a dynamic major and minor number. If none is available, the function fails.
pub fn alloc_dyn() -> Result<(u32, u32), ()> {
	// TODO
	Err(())
}

/// Frees the given pair of major/minor numbers `major` and `minor`.
pub fn free_dyn(major: u32, minor: u32) {
	// TODO
}*/

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
