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

//! Inner table implementation for hashmaps.

use crate::errno::AllocResult;
use alloc::alloc::Global;
use core::{
	alloc::{Allocator, Layout},
	borrow::Borrow,
	intrinsics::{likely, unlikely},
	iter::FusedIterator,
	marker::PhantomData,
	mem::{size_of, MaybeUninit},
	ops::BitAnd,
	ptr::NonNull,
	simd::{cmp::SimdPartialEq, u8x16},
};

/// Indicates a vacant entry in the map. This is a sentinel value for the lookup operation.
pub const CTRL_EMPTY: u8 = 0x80;
/// Indicates a deleted entry in the map.
pub const CTRL_DELETED: u8 = 0xfe;

/// The size of a group of entries.
pub const GROUP_SIZE: usize = 16;
/// The alignment for the allocation of the buffer in bytes.
const ALIGN: usize = 8;

/// For the given capacity, returns the size of the buffer and the offset of control blocks, both
/// in bytes.
fn buff_size<K, V>(capacity: usize) -> (usize, usize) {
	let ctrl_off = (capacity * size_of::<Slot<K, V>>()).next_multiple_of(GROUP_SIZE);
	let size = ctrl_off + capacity;
	(size, ctrl_off)
}

/// Initializes a new data buffer with the given minimum capacity and returns it along with its
/// actual capacity.
pub fn init_data<K, V>(capacity: usize) -> AllocResult<NonNull<u8>> {
	let (size, ctrl_off) = buff_size::<K, V>(capacity);
	unsafe {
		let layout = Layout::from_size_align_unchecked(size, ALIGN);
		let mut data = Global.allocate(layout)?;
		data.as_mut()[ctrl_off..].fill(CTRL_EMPTY);
		Ok(data.cast())
	}
}

/// Returns the slot part of the hash.
#[inline]
fn h1(hash: u64) -> u64 {
	hash >> 7
}

/// Returns the control part of the hash.
#[inline]
pub fn h2(hash: u64) -> u8 {
	(hash & 0x7f) as _
}

/// Returns the offset to a slot for the given `group` and in-group-index `index`.
#[inline]
pub fn get_slot_offset<K, V>(group: usize, index: usize) -> usize {
	(group * GROUP_SIZE + index) * size_of::<Slot<K, V>>()
}

/// Returns the group and in-group-index for the slot at the given offset.
#[inline]
pub fn get_slot_position<K, V>(off: usize) -> (usize, usize) {
	let off = off / size_of::<Slot<K, V>>();
	(off / GROUP_SIZE, off % GROUP_SIZE)
}

/// Iterator over set bits of the inner bitmask.
struct BitmaskIter(u16);

impl Iterator for BitmaskIter {
	type Item = usize;

	fn next(&mut self) -> Option<Self::Item> {
		let off = self.0.trailing_zeros();
		if off < 16 {
			self.0 &= !(1 << off);
			Some(off as _)
		} else {
			None
		}
	}
}

impl FusedIterator for BitmaskIter {}

/// Returns an iterator over the indexes of the elements that match `byte` in `group`.
#[inline]
fn group_match_byte(group: u8x16, byte: u8) -> impl Iterator<Item = usize> {
	let mask = u8x16::splat(byte);
	let matching = group.simd_eq(mask);
	BitmaskIter(matching.to_bitmask() as u16)
}

/// Returns the first empty element of the given `group`.
///
/// If `deleted` is set to `true`, the function also takes deleted entries into account.
#[inline]
pub fn group_match_unused(group: u8x16, deleted: bool) -> Option<usize> {
	let matching = if deleted {
		// Check for high bit set
		let mask = u8x16::splat(0x80);
		group.bitand(mask).simd_eq(mask)
	} else {
		let mask = u8x16::splat(CTRL_EMPTY);
		group.simd_eq(mask)
	};
	matching.first_set()
}

/// Returns an iterator over the indexes of the slots that are used in `group`.
///
/// The function ignores all used slots before `start`.
#[inline]
pub fn group_match_used(group: u8x16) -> impl Iterator<Item = usize> {
	let mask = u8x16::splat(0x80);
	let matching = group.bitand(mask).simd_ne(mask);
	BitmaskIter(matching.to_bitmask() as u16)
}

/// Internal representation of an entry.
pub struct Slot<K, V> {
	/// The key stored in the slot.
	pub key: MaybeUninit<K>,
	/// The value stored in the slot.
	pub value: MaybeUninit<V>,
}

/// Table inner to the hashmap, to handle the allocations and basic operations.
pub struct RawTable<K, V> {
	/// The allocated buffer.
	data: NonNull<u8>,
	/// The capacity of the table in number of elements.
	capacity: usize,

	_key: PhantomData<K>,
	_value: PhantomData<V>,
}

impl<K, V> Default for RawTable<K, V> {
	fn default() -> Self {
		Self::new()
	}
}

impl<K, V> RawTable<K, V> {
	/// Creates a new empty instance.
	pub const fn new() -> Self {
		Self {
			data: NonNull::dangling(),
			capacity: 0,

			_key: PhantomData,
			_value: PhantomData,
		}
	}

	/// Creates an instance with the given capacity in number of elements.
	pub fn with_capacity(capacity: usize) -> AllocResult<Self> {
		let capacity = capacity.next_multiple_of(GROUP_SIZE);
		Ok(Self {
			data: init_data::<K, V>(capacity)?,
			capacity,

			_key: PhantomData,
			_value: PhantomData,
		})
	}

	/// Returns the capacity of the table in number of elements.
	#[inline]
	pub fn capacity(&self) -> usize {
		self.capacity
	}

	/// Returns an immutable slice to the inner data.
	fn as_slice(&self) -> &[u8] {
		if self.capacity > 0 {
			let size = buff_size::<K, V>(self.capacity).0;
			unsafe { NonNull::slice_from_raw_parts(self.data, size).as_ref() }
		} else {
			&[]
		}
	}

	/// Returns a mutable slice to the inner data.
	fn as_mut_slice(&mut self) -> &mut [u8] {
		if self.capacity > 0 {
			let size = buff_size::<K, V>(self.capacity).0;
			unsafe { NonNull::slice_from_raw_parts(self.data, size).as_mut() }
		} else {
			&mut []
		}
	}

	/// Returns an immutable reference to the slot at the given offset in bytes.
	#[inline]
	pub fn get_slot(&self, off: usize) -> &Slot<K, V> {
		unsafe { &*(&self.as_slice()[off] as *const _ as *const _) }
	}

	/// Returns a mutable reference to the slot at the given offset in bytes.
	#[inline]
	pub fn get_slot_mut(&mut self, off: usize) -> &mut Slot<K, V> {
		unsafe { &mut *(&mut self.as_mut_slice()[off] as *mut _ as *mut _) }
	}

	/// Returns the control block for the given `group`.
	#[inline]
	pub fn get_ctrl(&self, group: usize) -> u8x16 {
		let off = buff_size::<K, V>(self.capacity).1 + group * GROUP_SIZE;
		let ctrl = &self.as_slice()[off..(off + GROUP_SIZE)];
		u8x16::from_slice(ctrl)
	}

	/// Sets the control bytes for a slot.
	#[inline]
	pub fn set_ctrl(&mut self, group: usize, index: usize, h2: u8) {
		let off = buff_size::<K, V>(self.capacity).1 + group * GROUP_SIZE + index;
		self.as_mut_slice()[off] = h2;
	}

	/// Returns the slot corresponding the given key and hash of the key.
	///
	/// The hash of the key is required to avoid computing it several times.
	///
	/// `deleted` tells whether the function might return deleted entries.
	///
	/// Return tuple:
	/// - The offset of the slot in the data buffer
	/// - Whether the slot is occupied
	pub fn find_slot<Q: ?Sized>(&self, key: &Q, hash: u64, deleted: bool) -> Option<(usize, bool)>
	where
		K: Borrow<Q>,
		Q: Eq,
	{
		let groups_count = self.capacity.div_ceil(GROUP_SIZE);
		if groups_count == 0 {
			return None;
		}
		let start_group = (h1(hash) % groups_count as u64) as usize;
		let mut group = start_group;
		let h2 = h2(hash);
		loop {
			// Find key in group
			let ctrl = self.get_ctrl(group);
			for i in group_match_byte(ctrl, h2) {
				let slot_off = get_slot_offset::<K, V>(group, i);
				let slot = self.get_slot(slot_off);
				let slot_key = unsafe { slot.key.assume_init_ref() };
				if likely(slot_key.borrow() == key) {
					return Some((slot_off, true));
				}
			}
			// Check for an empty slot
			if let Some(i) = group_match_unused(ctrl, deleted) {
				// TODO mark this line as cold (the #[cold] attribute works only on functions)
				return Some((get_slot_offset::<K, V>(group, i), false));
			}
			group = (group + 1) % groups_count;
			// If coming back to the first group
			if unlikely(group == start_group) {
				return None;
			}
		}
	}
}

impl<K, V> Drop for RawTable<K, V> {
	fn drop(&mut self) {
		let size = buff_size::<K, V>(self.capacity).0;
		unsafe {
			let layout = Layout::from_size_align_unchecked(size, ALIGN);
			Global.deallocate(self.data, layout)
		}
	}
}
