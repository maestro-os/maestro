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

//! A memory space is a virtual memory handler for a process. It handles virtual and physical
//! memory allocations for the process, as well as linkage between them.
//!
//! The memory space contains two types of structures:
//! - Mapping: A chunk of virtual memory that is allocated
//! - Gap: A chunk of virtual memory that is available to be allocated

pub mod copy;
mod gap;
mod mapping;
pub mod residence;
mod transaction;

use crate::{
	arch::x86::paging::{PAGE_FAULT_PRESENT, PAGE_FAULT_USER, PAGE_FAULT_WRITE},
	file::perm::AccessProfile,
	memory,
	memory::{vmem::VMem, VirtAddr, PROCESS_END},
};
use core::{
	alloc::AllocError,
	cmp::{min, Ordering},
	ffi::c_void,
	fmt,
	intrinsics::unlikely,
	mem,
	num::NonZeroUsize,
};
use gap::MemGap;
use mapping::MemMapping;
use residence::MapResidence;
use transaction::MemSpaceTransaction;
use utils::{
	collections::{btreemap::BTreeMap, vec::Vec},
	errno::{AllocResult, CollectResult, EResult},
	limits::PAGE_SIZE,
	TryClone,
};

/// Flag telling that a memory mapping can be written to.
pub const MAPPING_FLAG_WRITE: u8 = 0b00001;
/// Flag telling that a memory mapping can contain executable instructions.
pub const MAPPING_FLAG_EXEC: u8 = 0b00010;
/// Flag telling that a memory mapping is accessible from userspace.
pub const MAPPING_FLAG_USER: u8 = 0b00100;
/// Flag telling that a memory mapping has its physical memory shared with one
/// or more other mappings.
///
/// If the mapping is associated with a file, modifications made to the mapping are update to the
/// file.
pub const MAPPING_FLAG_SHARED: u8 = 0b1000;

/// The virtual address of the buffer used to map pages for copy.
const COPY_BUFFER: VirtAddr = VirtAddr(PROCESS_END.0 - PAGE_SIZE);

/// Tells whether the address is in bound of the userspace.
pub fn bound_check(addr: usize, n: usize) -> bool {
	addr >= PAGE_SIZE && addr.saturating_add(n) <= COPY_BUFFER.0
}

// TODO Add a variant for ASLR
/// Enumeration of constraints for the selection of the virtual address for a memory mapping.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MapConstraint {
	/// The mapping is done at a fixed address.
	///
	/// Previous allocation(s) in the range of the allocation are unmapped.
	///
	/// The allocation is allowed to take place outside ranges that are normally allowed, but not
	/// in kernelspace.
	Fixed(VirtAddr),

	/// Providing a hint for the address to use. The allocator will try to use the address if
	/// available.
	///
	/// If not available, the constraint is ignored and another address is selected.
	Hint(VirtAddr),

	/// No constraint.
	None,
}

impl MapConstraint {
	/// Tells whether the constraint is valid.
	pub fn is_valid(self) -> bool {
		match self {
			// Checking the address is within userspace is required because `Fixed` allocations can
			// take place *outside of gaps* but *not inside the kernelspace*
			MapConstraint::Fixed(addr) => {
				// The copy buffer is located right before the kernelspace
				addr < COPY_BUFFER && addr.is_aligned_to(PAGE_SIZE)
			}
			MapConstraint::Hint(addr) => addr.is_aligned_to(PAGE_SIZE),
			_ => true,
		}
	}
}

/// Removes gaps in `on` in the given range, using `transaction`.
///
/// `start` is the start address of the range and `size` is the size of the range in pages.
fn remove_gaps_in_range(
	transaction: &mut MemSpaceTransaction,
	start: VirtAddr,
	size: usize,
) -> AllocResult<()> {
	// Start the search at the gap containing the start address
	let search_start = transaction
		.mem_space_state
		.get_gap_for_addr(start)
		.map(MemGap::get_begin)
		// No gap contain the start address, start at the next one
		.unwrap_or(start);
	// Bound the search to the end of the range
	let end = start + size * PAGE_SIZE;
	// Collect gaps that match
	let gaps = transaction
		.mem_space_state
		.gaps
		.range(search_start..end)
		.map(|(_, b)| b.clone())
		.collect::<CollectResult<Vec<_>>>()
		.0?;
	// Iterate on gaps and apply modifications
	for gap in gaps {
		let gap_begin = gap.get_begin();
		let gap_end = gap.get_end();
		// Compute range to remove
		let start = start.0.clamp(gap_begin.0, gap_end.0);
		let end = end.0.clamp(gap_begin.0, gap_end.0);
		// Rounding is not a problem because all values are multiples of the page size
		let size = (end - start) / PAGE_SIZE;
		// Consume the gap and store new gaps
		let (prev, next) = gap.consume(start, size);
		transaction.remove_gap(gap_begin)?;
		if let Some(g) = prev {
			transaction.insert_gap(g)?;
		}
		if let Some(g) = next {
			transaction.insert_gap(g)?;
		}
	}
	Ok(())
}

/// Inner state of the memory space, to use as a model for the virtual memory context.
#[derive(Debug, Default)]
struct MemSpaceState {
	/// Binary tree storing the list of memory gaps, ready for new mappings.
	///
	/// The collection is sorted by pointer to the beginning of the mapping on the virtual
	/// memory.
	gaps: BTreeMap<VirtAddr, MemGap>,
	/// Binary tree storing the list of memory mappings.
	///
	/// Sorted by pointer to the beginning of the mapping on the virtual memory.
	mappings: BTreeMap<*mut u8, MemMapping>,

	/// The number of used virtual memory pages.
	vmem_usage: usize,

	/// The initial pointer of the `[s]brk` system calls.
	brk_init: VirtAddr,
	/// The current pointer of the `[s]brk` system calls.
	brk_addr: VirtAddr,
}

impl MemSpaceState {
	/// Returns a reference to a gap with at least size `size`.
	///
	/// `size` is the minimum size of the gap to be returned.
	///
	/// If no gap large enough is available, the function returns `None`.
	fn get_gap(&self, size: NonZeroUsize) -> Option<&MemGap> {
		self.gaps
			.iter()
			.map(|(_, g)| g)
			.find(|g| g.get_size() >= size)
	}

	/// Comparison function to search for the object containing the address `addr`.
	///
	/// Arguments:
	/// - `begin` is the beginning of the object to compare for
	/// - `size` is the size of the object in pages
	fn addr_search(begin: VirtAddr, size: usize, addr: VirtAddr) -> Ordering {
		let end = begin + size * PAGE_SIZE;
		if addr >= begin && addr < end {
			Ordering::Equal
		} else if addr < begin {
			Ordering::Less
		} else {
			Ordering::Greater
		}
	}

	/// Returns a reference to the gap containing the given virtual address.
	///
	/// If no gap contain the pointer, the function returns `None`.
	fn get_gap_for_addr(&self, addr: VirtAddr) -> Option<&MemGap> {
		self.gaps
			.cmp_get(|key, value| Self::addr_search(*key, value.get_size().get(), addr))
	}

	/// Returns an immutable reference to the memory mapping containing the given virtual
	/// address.
	///
	/// If no mapping contains the address, the function returns `None`.
	pub fn get_mapping_for_addr(&self, addr: VirtAddr) -> Option<&MemMapping> {
		self.mappings.cmp_get(|key, value| {
			Self::addr_search(VirtAddr::from(*key), value.get_size().get(), addr)
		})
	}

	/// Returns a mutable reference to the memory mapping containing the given virtual
	/// address.
	///
	/// If no mapping contains the address, the function returns `None`.
	pub fn get_mut_mapping_for_addr(&mut self, addr: VirtAddr) -> Option<&mut MemMapping> {
		self.mappings.cmp_get_mut(|key, value| {
			Self::addr_search(VirtAddr::from(*key), value.get_size().get(), addr)
		})
	}
}

/// A virtual memory space.
pub struct MemSpace {
	/// The memory space's structure, used as a model for `vmem`.
	state: MemSpaceState,
	/// Architecture-specific virtual memory context handler.
	vmem: VMem,
}

impl MemSpace {
	/// Creates a new virtual memory object.
	pub fn new() -> AllocResult<Self> {
		let mut s = Self {
			state: MemSpaceState::default(),
			vmem: VMem::new()?,
		};
		// Create the default gap of memory which is present at the beginning
		let begin = memory::ALLOC_BEGIN;
		let size = (COPY_BUFFER.0 - begin.0) / PAGE_SIZE;
		let gap = MemGap::new(begin, NonZeroUsize::new(size).unwrap());
		let mut transaction = MemSpaceTransaction::new(&mut s.state, &mut s.vmem);
		transaction.insert_gap(gap)?;
		transaction.commit();
		Ok(s)
	}

	/// Returns an immutable reference to the virtual memory context.
	#[inline]
	pub fn get_vmem(&self) -> &VMem {
		&self.vmem
	}

	/// Returns the number of virtual memory pages in the memory space.
	#[inline]
	pub fn get_vmem_usage(&self) -> usize {
		self.state.vmem_usage
	}

	/// Returns an immutable reference to the memory mapping containing the given virtual
	/// address.
	///
	/// If no mapping contains the address, the function returns `None`.
	#[inline]
	pub fn get_mapping_for_addr(&self, addr: VirtAddr) -> Option<&MemMapping> {
		self.state.get_mapping_for_addr(addr)
	}

	/// Maps a chunk of memory.
	///
	/// The function has complexity `O(log n)`.
	///
	/// Arguments:
	/// - `map_constraint` is the constraint to fulfill for the allocation.
	/// - `size` represents the size of the mapping in number of memory pages.
	/// - `flags` represents the flags for the mapping.
	/// - `residence` is the residence of the mapping to be created.
	///
	/// The underlying physical memory is not allocated directly but only when an attempt to write
	/// the memory is detected.
	///
	/// On success, the function returns a pointer to the newly mapped virtual memory.
	///
	/// If the given pointer is not page-aligned, the function returns an error.
	pub fn map(
		&mut self,
		map_constraint: MapConstraint,
		size: NonZeroUsize,
		flags: u8,
		residence: MapResidence,
	) -> AllocResult<*mut u8> {
		if !map_constraint.is_valid() {
			return Err(AllocError);
		}
		let mut transaction = MemSpaceTransaction::new(&mut self.state, &mut self.vmem);
		// Get suitable gap for the given constraint
		let (gap, off) = match map_constraint {
			MapConstraint::Fixed(addr) => {
				Self::unmap_impl(&mut transaction, addr, size, true)?;
				// Remove gaps that are present where the mapping is to be placed
				remove_gaps_in_range(&mut transaction, addr, size.get())?;
				// Create a fictive gap. This is required because fixed allocations may be used
				// outside allowed gaps
				let gap = MemGap::new(addr, size);
				(gap, 0)
			}
			MapConstraint::Hint(addr) => {
				transaction
					.mem_space_state
					// Get the gap for the pointer
					.get_gap_for_addr(addr)
					.and_then(|gap| {
						// Offset in the gap
						let off = gap.get_page_offset_for(addr);
						// Check whether the mapping fits in the gap
						let end = off.checked_add(size.get())?;
						(end <= gap.get_size().get()).then_some((gap.clone(), off))
					})
					// Hint cannot be satisfied. Get a large enough gap
					.or_else(|| {
						let gap = transaction.mem_space_state.get_gap(size)?;
						Some((gap.clone(), 0))
					})
					.ok_or(AllocError)?
					.clone()
			}
			MapConstraint::None => {
				let gap = transaction
					.mem_space_state
					.get_gap(size)
					.ok_or(AllocError)?
					.clone();
				(gap, 0)
			}
		};
		let addr = (gap.get_begin() + off * PAGE_SIZE).as_ptr();
		// Split the old gap to fit the mapping, and insert new gaps
		let (left_gap, right_gap) = gap.consume(off, size.get());
		transaction.remove_gap(gap.get_begin())?;
		if let Some(new_gap) = left_gap {
			transaction.insert_gap(new_gap)?;
		}
		if let Some(new_gap) = right_gap {
			transaction.insert_gap(new_gap)?;
		}
		// Create the mapping
		let m = MemMapping::new(addr, size, flags, residence)?;
		transaction.insert_mapping(m)?;
		transaction.commit();
		Ok(addr)
	}

	/// Implementation for `unmap`.
	///
	/// If `nogap` is `true`, the function does not create any gap.
	///
	/// On success, the function returns the transaction.
	fn unmap_impl(
		transaction: &mut MemSpaceTransaction,
		addr: VirtAddr,
		size: NonZeroUsize,
		nogap: bool,
	) -> AllocResult<()> {
		// Remove every mapping in the chunk to unmap
		let mut i = 0;
		while i < size.get() {
			// The current page's beginning
			let page_addr = addr + i * PAGE_SIZE;
			// The mapping containing the page
			let Some(mapping) = transaction.mem_space_state.get_mapping_for_addr(page_addr) else {
				// TODO jump to next mapping directly using binary tree (currently O(n log n))
				i += 1;
				continue;
			};
			// The pointer to the beginning of the mapping
			let mapping_begin = mapping.get_begin();
			// The offset in the mapping to the beginning of pages to unmap
			let inner_off = (page_addr.0 - mapping_begin as usize) / PAGE_SIZE;
			// The number of pages to unmap in the mapping
			let pages = min(size.get() - i, mapping.get_size().get() - inner_off);
			i += pages;
			// Newly created mappings and gap after removing parts of the previous one
			let (prev, gap, next) = mapping.split(inner_off, pages)?;
			// Remove the old mapping and insert new ones
			transaction.remove_mapping(mapping_begin)?;
			if let Some(m) = prev {
				transaction.insert_mapping(m)?;
			}
			if let Some(m) = next {
				transaction.insert_mapping(m)?;
			}
			if nogap {
				continue;
			}
			// Insert gap
			if let Some(mut gap) = gap {
				// Merge previous gap
				let prev_gap = (!gap.get_begin().is_null())
					.then(|| {
						let prev_gap_ptr = gap.get_begin() - 1;
						transaction.mem_space_state.get_gap_for_addr(prev_gap_ptr)
					})
					.flatten()
					.cloned();
				if let Some(p) = prev_gap {
					transaction.remove_gap(p.get_begin())?;
					gap.merge(&p);
				}
				// Merge next gap
				let next_gap = transaction
					.mem_space_state
					.get_gap_for_addr(gap.get_end())
					.cloned();
				if let Some(n) = next_gap {
					transaction.remove_gap(n.get_begin())?;
					gap.merge(&n);
				}
				transaction.insert_gap(gap)?;
			}
		}
		Ok(())
	}

	/// Unmaps the given mapping of memory.
	///
	/// Arguments:
	/// - `addr` represents the aligned address of the beginning of the chunk to unmap.
	/// - `size` represents the size of the mapping in number of memory pages.
	/// - `brk` tells whether the function is called through the `brk` syscall.
	///
	/// The function frees the physical memory the mapping points to
	/// unless shared by one or several other memory mappings.
	///
	/// After this function returns, the access to the mapping of memory shall
	/// be revoked and further attempts to access it shall result in a page
	/// fault.
	#[allow(clippy::not_unsafe_ptr_arg_deref)]
	pub fn unmap(&mut self, addr: VirtAddr, size: NonZeroUsize, brk: bool) -> AllocResult<()> {
		// Validation
		if unlikely(!addr.is_aligned_to(PAGE_SIZE)) {
			return Err(AllocError);
		}
		let mut transaction = MemSpaceTransaction::new(&mut self.state, &mut self.vmem);
		// Do not create gaps if unmapping for `*brk` system calls as this space is reserved by
		// it and must not be reused by `mmap`
		Self::unmap_impl(&mut transaction, addr, size, brk)?;
		transaction.commit();
		Ok(())
	}

	/// Binds the memory space to the current kernel.
	pub fn bind(&self) {
		self.vmem.bind();
	}

	/// Tells whether the memory space is bound.
	pub fn is_bound(&self) -> bool {
		self.vmem.is_bound()
	}

	/// Clones the current memory space for process forking.
	pub fn fork(&mut self) -> AllocResult<MemSpace> {
		// Clone gaps
		let gaps = self.state.gaps.try_clone()?;
		// Clone vmem and mappings and update them for COW
		let mut new_vmem = VMem::new()?;
		let mut vmem_transaction = self.vmem.transaction();
		let mut new_vmem_transaction = new_vmem.transaction();
		let mappings = self
			.state
			.mappings
			.iter_mut()
			.map(|(p, mapping)| {
				let mut new_mapping = mapping.try_clone()?;
				mapping.apply_to(&mut vmem_transaction)?;
				new_mapping.apply_to(&mut new_vmem_transaction)?;
				Ok((*p, new_mapping))
			})
			.collect::<AllocResult<CollectResult<_>>>()?
			.0?;
		// No fallible operation left, commit
		new_vmem_transaction.commit();
		vmem_transaction.commit();
		drop(new_vmem_transaction);
		drop(vmem_transaction);
		Ok(Self {
			state: MemSpaceState {
				gaps,
				mappings,

				vmem_usage: self.state.vmem_usage,

				brk_init: self.state.brk_init,
				brk_addr: self.state.brk_addr,
			},
			vmem: new_vmem,
		})
	}

	/// Allocates the physical pages on the given range.
	///
	/// Arguments:
	/// - `addr` is the virtual address to beginning of the range to allocate.
	/// - `len` is the size of the range in bytes.
	///
	/// If the mapping does not exist, the function returns an error.
	///
	/// On error, allocations that have been made are not freed as it does not affect the behaviour
	/// from the user's point of view.
	pub fn alloc(&mut self, addr: VirtAddr, len: usize) -> AllocResult<()> {
		let mut transaction = self.vmem.transaction();
		let mut off = 0;
		while off < len {
			let addr = addr + off;
			if let Some(mapping) = self.state.get_mut_mapping_for_addr(addr) {
				let page_offset = (addr.0 - mapping.get_begin() as usize) / PAGE_SIZE;
				mapping.alloc(page_offset, &mut transaction)?;
			}
			off += PAGE_SIZE;
		}
		transaction.commit();
		Ok(())
	}

	/// Sets protection for the given range of memory.
	///
	/// Arguments:
	/// - `addr` is the address to the beginning of the range to be set
	/// - `len` is the length of the range in bytes
	/// - `prot` is a set of mapping flags
	/// - `access_profile` is the access profile to check permissions
	///
	/// If a mapping to be modified is associated with a file, and the file doesn't have the
	/// matching permissions, the function returns an error.
	pub fn set_prot(
		&mut self,
		_addr: *mut c_void,
		_len: usize,
		_prot: u8,
		_access_profile: &AccessProfile,
	) -> EResult<()> {
		// TODO Iterate on mappings in the range:
		//		If the mapping is shared and associated to a file, check file permissions match
		// `prot` (only write)
		//		Split the mapping if needed
		//		Set permissions
		//		Update vmem
		Ok(())
	}

	/// Returns the address for the `brk` syscall.
	pub fn get_brk(&self) -> VirtAddr {
		self.state.brk_addr
	}

	/// Sets the initial pointer for the `brk` syscall.
	///
	/// This function MUST be called *only once*, before the program starts.
	///
	/// `addr` MUST be page-aligned.
	pub fn set_brk_init(&mut self, addr: VirtAddr) {
		debug_assert!(addr.is_aligned_to(PAGE_SIZE));
		self.state.brk_init = addr;
		self.state.brk_addr = addr;
	}

	/// Sets the address for the `brk` syscall.
	///
	/// If the memory cannot be allocated, the function returns an error.
	#[allow(clippy::not_unsafe_ptr_arg_deref)]
	pub fn set_brk(&mut self, addr: VirtAddr) -> AllocResult<()> {
		if addr >= self.state.brk_addr {
			// Check the pointer is valid
			if addr > COPY_BUFFER {
				return Err(AllocError);
			}
			// Allocate memory
			let begin = self.state.brk_addr.align_to(PAGE_SIZE);
			let pages = (addr.0 - begin.0).div_ceil(PAGE_SIZE);
			let Some(pages) = NonZeroUsize::new(pages) else {
				return Ok(());
			};
			let flags = MAPPING_FLAG_WRITE | MAPPING_FLAG_USER;
			self.map(
				MapConstraint::Fixed(begin),
				pages,
				flags,
				MapResidence::Normal,
			)?;
		} else {
			// Check the pointer is valid
			if unlikely(addr < self.state.brk_init) {
				return Err(AllocError);
			}
			// Free memory
			let begin = addr.align_to(PAGE_SIZE);
			let pages = (begin.0 - addr.0).div_ceil(PAGE_SIZE);
			let Some(pages) = NonZeroUsize::new(pages) else {
				return Ok(());
			};
			self.unmap(begin, pages, true)?;
		}
		self.state.brk_addr = addr;
		Ok(())
	}

	/// Function called whenever the CPU triggered a page fault for the context.
	///
	/// This function determines whether the process should continue or not.
	///
	/// If continuing, the function must resolve the issue before returning.
	/// A typical situation where is function is useful is for Copy-On-Write allocations.
	///
	/// Arguments:
	/// - `addr` is the virtual address of the wrong memory access that caused the fault.
	/// - `code` is the error code given along with the error.
	///
	/// If the process should continue, the function returns `true`, else `false`.
	pub fn handle_page_fault(&mut self, addr: VirtAddr, code: u32) -> bool {
		if code & PAGE_FAULT_PRESENT == 0 {
			return false;
		}
		let Some(mapping) = self.state.get_mut_mapping_for_addr(addr) else {
			return false;
		};
		// Check permissions
		let code_write = code & PAGE_FAULT_WRITE != 0;
		let mapping_write = mapping.get_flags() & MAPPING_FLAG_WRITE != 0;
		if code_write && !mapping_write {
			return false;
		}
		// TODO check exec
		let code_userspace = code & PAGE_FAULT_USER != 0;
		let mapping_userspace = mapping.get_flags() & MAPPING_FLAG_USER != 0;
		if code_userspace && !mapping_userspace {
			return false;
		}
		// Map the accessed page
		let page_offset = (addr.0 - mapping.get_begin() as usize) / PAGE_SIZE;
		let mut transaction = self.vmem.transaction();
		// TODO use OOM killer
		mapping
			.alloc(page_offset, &mut transaction)
			.expect("Out of memory!");
		transaction.commit();
		true
	}
}

impl fmt::Debug for MemSpace {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(&self.state, f)
	}
}

impl Drop for MemSpace {
	fn drop(&mut self) {
		// Synchronize all mappings to disk
		let mappings = mem::take(&mut self.state.mappings);
		for (_, m) in mappings {
			// Ignore I/O errors
			let _ = m.fs_sync(&self.vmem);
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn test0() {
		let mut mem_space = MemSpace::new().unwrap();
		let addr = VirtAddr(0x1000);
		let size = NonZeroUsize::new(1).unwrap();
		let res = mem_space
			.map(
				MapConstraint::Fixed(addr),
				size,
				MAPPING_FLAG_WRITE | MAPPING_FLAG_USER,
				MapResidence::Normal,
			)
			.unwrap();
		assert_eq!(VirtAddr::from(res), addr);
		// TODO test access
		/*assert!(!mem_space.can_access(null(), PAGE_SIZE, true, true));
		assert!(!mem_space.can_access(null(), PAGE_SIZE + 1, true, true));
		assert!(mem_space.can_access(addr as _, PAGE_SIZE, true, true));
		assert!(!mem_space.can_access(addr as _, PAGE_SIZE + 1, true, true));*/
		mem_space.unmap(addr, size, false).unwrap();
		//assert!(!mem_space.can_access(addr as _, PAGE_SIZE, true, true));
	}
}
