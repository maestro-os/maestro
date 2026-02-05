/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! This module handles minor/major numbers, including their allocation.

use crate::{
	device::DeviceType,
	sync::{once::OnceInit, spin::Spin},
};
use utils::{collections::id_allocator::IDAllocator, errno::AllocResult};

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

/// Minimal dynamically allocatable major number
pub const DYN_MAJOR_MIN: u32 = 234;
/// Maximal dynamically allocatable major number
pub const DYN_MAJOR_MAX: u32 = 255;

const DYN_MAJOR_CNT: usize = (DYN_MAJOR_MAX - DYN_MAJOR_MIN + 1) as usize;

/// Major numbers allocator
static BLOCK_MAJOR_ALLOCATOR: Spin<IDAllocator<[u8; DYN_MAJOR_CNT / 8]>> =
	Spin::new(IDAllocator::new_inplace());
/// Major numbers allocator
static CHAR_MAJOR_ALLOCATOR: Spin<IDAllocator<[u8; DYN_MAJOR_CNT / 8]>> =
	Spin::new(IDAllocator::new_inplace());

fn get_allocator(device_type: DeviceType) -> &'static Spin<IDAllocator<[u8; DYN_MAJOR_CNT / 8]>> {
	match device_type {
		DeviceType::Block => &BLOCK_MAJOR_ALLOCATOR,
		DeviceType::Char => &CHAR_MAJOR_ALLOCATOR,
	}
}

/// A block of minor numbers associated with a unique major number, allowing dynamic allocations of
/// minor numbers.
pub struct MajorBlock {
	/// The device type.
	device_type: DeviceType,
	/// The block's major number.
	major: u32,

	/// The minor number allocator.
	allocator: IDAllocator,
}

impl MajorBlock {
	/// Creates a new instance with the given major number `major`
	///
	/// This function may overlap with other allocated majors. Avoiding overlaps is the caller's
	/// responsibility.
	pub fn new_fixed(device_type: DeviceType, major: u32) -> AllocResult<Self> {
		Ok(Self {
			device_type,
			major,

			allocator: IDAllocator::new_allocated(512)?,
		})
	}

	/// Creates a new instance with a dynamically allocated major number.
	pub fn new_dyn(device_type: DeviceType) -> AllocResult<Self> {
		let mut major_allocator = get_allocator(device_type).lock();
		let major = DYN_MAJOR_MIN + major_allocator.alloc(None)?;
		Ok(Self {
			device_type,
			major,

			allocator: IDAllocator::new_allocated(512)?,
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
		if (DYN_MAJOR_MIN..=DYN_MAJOR_MAX).contains(&self.major) {
			let major = self.major - DYN_MAJOR_MIN;
			let mut major_allocator = get_allocator(self.device_type).lock();
			major_allocator.free(major);
		}
	}
}

/// The major number block for additional block device partitions
pub const BLOCK_EXTENDED_MAJOR: u32 = 259;

/// Handle for allocations in the major number block for additional block device partitions
pub static BLOCK_EXTENDED_MAJOR_HANDLE: OnceInit<Spin<MajorBlock>> = unsafe { OnceInit::new() };

pub(super) fn init() -> AllocResult<()> {
	let major = MajorBlock::new_fixed(DeviceType::Block, BLOCK_EXTENDED_MAJOR)?;
	unsafe {
		OnceInit::init(&BLOCK_EXTENDED_MAJOR_HANDLE, Spin::new(major));
	}
	Ok(())
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn device_number() {
		for maj in 0..512 {
			for min in 0..512 {
				let dev = makedev(maj, min);
				assert_eq!(major(dev), maj);
				assert_eq!(minor(dev), min);
			}
		}
	}
}
