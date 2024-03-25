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

mod gap;
mod mapping;
pub mod ptr;
pub mod residence;
mod transaction;

use crate::{
	file::perm::AccessProfile,
	memory,
	memory::{vmem, vmem::VMem},
	process::mem_space::residence::Page,
};
use core::{
	alloc::AllocError,
	cmp::{min, Ordering},
	ffi::c_void,
	fmt, mem,
	num::NonZeroUsize,
	ptr::null_mut,
};
use gap::MemGap;
use mapping::MemMapping;
use residence::MapResidence;
use transaction::MemSpaceTransaction;
use utils::{
	collections::{btreemap::BTreeMap, vec::Vec},
	errno::{AllocResult, CollectResult, EResult},
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
/// TODO use PROCESS_END instead of hardcoding value
const COPY_BUFFER: *mut Page = (0xc0000000 - memory::PAGE_SIZE) as _;

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
	Fixed(*mut c_void),

	/// Providing a hint for the address to use. The allocator will try to use the address if
	/// available.
	///
	/// If not available, the constraint is ignored and another address is selected.
	Hint(*mut c_void),

	/// No constraint.
	None,
}

impl MapConstraint {
	/// Tells whether the constraint is valid.
	pub fn is_valid(&self) -> bool {
		match self {
			// Checking the address is within userspace is required because `Fixed` allocations can
			// take place *outside of gaps* but *not inside the kernelspace*
			MapConstraint::Fixed(addr) => {
				*addr < COPY_BUFFER as _ && addr.is_aligned_to(memory::PAGE_SIZE)
			}
			MapConstraint::Hint(addr) => addr.is_aligned_to(memory::PAGE_SIZE),
			_ => true,
		}
	}
}

/// Removes gaps in `on` in the given range, using `transaction`.
///
/// `start` is the start address of the range and `size` is the size of the range in pages.
fn remove_gaps_in_range(
	transaction: &mut MemSpaceTransaction,
	start: *const c_void,
	size: usize,
) -> AllocResult<()> {
	// Start the search at the gap containing the start address
	let search_start = transaction
		.mem_space_state
		.get_gap_for_ptr(start)
		.map(MemGap::get_begin)
		// No gap contain the start address, start at the next one
		.unwrap_or(start);
	// Bound the search to the end of the range
	let end = (start as usize + size * memory::PAGE_SIZE) as *const c_void;
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
		let start = (start as usize).clamp(gap_begin as usize, gap_end as usize);
		let end = (end as usize).clamp(gap_begin as usize, gap_end as usize);
		// Rounding is not a problem because all values are multiples of the page size
		let size = (end - start) / memory::PAGE_SIZE;
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
#[derive(Debug)]
struct MemSpaceState {
	/// Binary tree storing the list of memory gaps, ready for new mappings.
	///
	/// The collection is sorted by pointer to the beginning of the mapping on the virtual
	/// memory.
	gaps: BTreeMap<*const c_void, MemGap>,
	/// Binary tree storing the list of memory mappings.
	///
	/// Sorted by pointer to the beginning of the mapping on the virtual memory.
	mappings: BTreeMap<*const c_void, MemMapping>,

	/// The number of used virtual memory pages.
	vmem_usage: usize,

	/// The initial pointer of the `[s]brk` system calls.
	brk_init: *mut c_void,
	/// The current pointer of the `[s]brk` system calls.
	brk_ptr: *mut c_void,
}

impl Default for MemSpaceState {
	fn default() -> Self {
		Self {
			gaps: Default::default(),
			mappings: Default::default(),

			vmem_usage: 0,

			brk_init: null_mut::<_>(),
			brk_ptr: null_mut::<_>(),
		}
	}
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

	/// Comparison function to search for the object containing `ptr`.
	///
	/// Arguments:
	/// - `begin` is the beginning of the object to compare for
	/// - `size` is the size of the object in pages
	fn ptr_search(begin: *const c_void, size: usize, ptr: *const c_void) -> Ordering {
		let begin = begin as usize;
		let end = begin + size * memory::PAGE_SIZE;
		let ptr = ptr as usize;
		if ptr >= begin && ptr < end {
			Ordering::Equal
		} else if ptr < begin {
			Ordering::Less
		} else {
			Ordering::Greater
		}
	}

	/// Returns a reference to the gap containing the given virtual address `ptr`.
	///
	/// If no gap contain the pointer, the function returns `None`.
	fn get_gap_for_ptr(&self, ptr: *const c_void) -> Option<&MemGap> {
		self.gaps
			.cmp_get(|key, value| Self::ptr_search(*key, value.get_size().get(), ptr))
	}

	/// Returns an immutable reference to the memory mapping containing the given virtual
	/// address `ptr`.
	///
	/// If no mapping contains the address, the function returns `None`.
	pub fn get_mapping_for_ptr(&self, ptr: *const c_void) -> Option<&MemMapping> {
		self.mappings
			.cmp_get(|key, value| Self::ptr_search(*key, value.get_size().get(), ptr))
	}

	/// Returns a mutable reference to the memory mapping containing the given virtual
	/// address `ptr`.
	///
	/// If no mapping contains the address, the function returns `None`.
	pub fn get_mut_mapping_for_ptr(&mut self, ptr: *const c_void) -> Option<&mut MemMapping> {
		self.mappings
			.cmp_get_mut(|key, value| Self::ptr_search(*key, value.get_size().get(), ptr))
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
		let size = (COPY_BUFFER as usize - begin as usize) / memory::PAGE_SIZE;
		let gap = MemGap::new(begin, NonZeroUsize::new(size).unwrap());
		let mut transaction = MemSpaceTransaction::new(&mut s.state, &mut s.vmem);
		transaction.insert_gap(gap)?;
		transaction.commit();
		drop(transaction);
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
	/// address `ptr`.
	///
	/// If no mapping contains the address, the function returns `None`.
	#[inline]
	pub fn get_mapping_for_ptr(&self, ptr: *const c_void) -> Option<&MemMapping> {
		self.state.get_mapping_for_ptr(ptr)
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
	) -> AllocResult<*mut c_void> {
		if !map_constraint.is_valid() {
			return Err(AllocError);
		}
		let mut transaction = MemSpaceTransaction::new(&mut self.state, &mut self.vmem);
		// Get gap suitable for the given constraint
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
					.get_gap_for_ptr(addr)
					.and_then(|gap| {
						// Offset in the gap
						let off = gap.get_page_offset_for(addr);
						// Check whether the mapping fits in the gap
						let end = off.checked_add(size.get())?;
						(end <= gap.get_size().get()).then_some((gap.clone(), off))
					})
					// Hint cannot be satisfied. Get a gap large enough
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
		let addr = (gap.get_begin() as usize + (off * memory::PAGE_SIZE)) as *mut c_void;
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
		ptr: *const c_void,
		size: NonZeroUsize,
		nogap: bool,
	) -> AllocResult<()> {
		// Remove every mapping in the chunk to unmap
		let mut i = 0;
		while i < size.get() {
			// The current page's beginning
			let page_ptr = (ptr as usize + i * memory::PAGE_SIZE) as *const c_void;
			// The mapping containing the page
			let Some(mapping) = transaction.mem_space_state.get_mapping_for_ptr(page_ptr) else {
				// TODO jump to next mapping directly using binary tree (currently O(n log n))
				i += 1;
				continue;
			};
			// The pointer to the beginning of the mapping
			let mapping_begin = mapping.get_begin();
			// The offset in the mapping to the beginning of pages to unmap
			let begin = (page_ptr as usize - mapping_begin as usize) / memory::PAGE_SIZE;
			// The number of pages to unmap in the mapping
			let pages = min(size.get() - i, mapping.get_size().get() - begin);
			i += pages;
			// Newly created mappings and gap after removing parts of the previous one
			let (prev, gap, next) = mapping.split(begin, pages)?;
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
						let prev_gap_ptr = unsafe { gap.get_begin().sub(1) };
						transaction.mem_space_state.get_gap_for_ptr(prev_gap_ptr)
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
					.get_gap_for_ptr(gap.get_end())
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
	/// - `ptr` represents the aligned address of the beginning of the chunk to unmap.
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
	pub fn unmap(&mut self, ptr: *const c_void, size: NonZeroUsize, brk: bool) -> AllocResult<()> {
		if !ptr.is_aligned_to(memory::PAGE_SIZE) {
			return Err(AllocError);
		}
		let mut transaction = MemSpaceTransaction::new(&mut self.state, &mut self.vmem);
		// Do not create gaps if unmapping for `*brk` system calls as this space is reserved by
		// it and must not be reused by `mmap`
		Self::unmap_impl(&mut transaction, ptr, size, brk)?;
		transaction.commit();
		Ok(())
	}

	/// Same as `map`, except the function returns a pointer to the end of the
	/// memory mapping.
	pub fn map_stack(&mut self, size: NonZeroUsize, flags: u8) -> AllocResult<*mut c_void> {
		let mapping_ptr = self.map(MapConstraint::None, size, flags, MapResidence::Normal)?;
		Ok((mapping_ptr as usize + (size.get() * memory::PAGE_SIZE)) as _)
	}

	/// Same as `unmap`, except the function takes a pointer to the end of the
	/// memory mapping.
	#[allow(clippy::not_unsafe_ptr_arg_deref)]
	pub fn unmap_stack(&mut self, ptr: *const c_void, size: NonZeroUsize) -> AllocResult<()> {
		// Safe because the new pointer stays in the range of the allocated mapping
		let ptr = (ptr as usize - (size.get() * memory::PAGE_SIZE)) as _;
		self.unmap(ptr, size, false)
	}

	// TODO Optimize (use MMU)
	/// Tells whether the given mapping of memory `ptr` of size `size` in bytes
	/// can be accessed.
	///
	/// Arguments:
	/// - `user` tells whether the memory must be accessible from userspace or just kernelspace.
	/// - `write` tells whether to check for write permission.
	pub fn can_access(&self, ptr: *const u8, size: usize, user: bool, write: bool) -> bool {
		// TODO Allow reading kernelspace data that is available to userspace?
		let mut i = 0;
		while i < size {
			// The beginning of the current page
			let p = (ptr as usize + i) as _;
			let Some(mapping) = self.state.get_mapping_for_ptr(p) else {
				return false;
			};
			// Check mapping's flags
			let flags = mapping.get_flags();
			if write && (flags & MAPPING_FLAG_WRITE == 0) {
				return false;
			}
			if user && (flags & MAPPING_FLAG_USER == 0) {
				return false;
			}
			i += mapping.get_size().get() * memory::PAGE_SIZE;
		}
		true
	}

	// TODO Optimize (use MMU)
	/// Tells whether the given zero-terminated string beginning at `ptr` can be
	/// accessed.
	///
	/// Arguments:
	/// - `user` tells whether the memory must be accessible from userspace or just kernelspace.
	/// - `write` tells whether to check for write permission.
	///
	/// If the memory can be accessed, the function returns the length of the string located at
	/// the pointer `ptr`.
	///
	/// If the memory cannot be accessed, the function returns `None`.
	#[allow(clippy::not_unsafe_ptr_arg_deref)]
	pub fn can_access_string(&self, ptr: *const u8, user: bool, write: bool) -> Option<usize> {
		// TODO Allow reading kernelspace data that is available to userspace?
		unsafe {
			vmem::switch(&self.vmem, move || {
				let mut i = 0;
				'outer: loop {
					// Safe because not dereferenced before checking if accessible
					let curr_ptr = ptr.add(i);
					let mapping = self.state.get_mapping_for_ptr(curr_ptr as _)?;
					// Check mapping flags
					let flags = mapping.get_flags();
					if write && (flags & MAPPING_FLAG_WRITE == 0) {
						return None;
					}
					if user && (flags & MAPPING_FLAG_USER == 0) {
						return None;
					}
					// The beginning of the current page
					let page_begin = utils::down_align(curr_ptr as _, memory::PAGE_SIZE);
					// The offset of the current pointer in its page
					let inner_off = curr_ptr as usize - page_begin as usize;
					let check_size = memory::PAGE_SIZE - inner_off;
					// Look for the null byte
					for j in 0..check_size {
						let c = *curr_ptr.add(j);
						// TODO Optimize by checking several bytes at a time
						if c == b'\0' {
							break 'outer;
						}
						i += 1;
					}
				}
				Some(i)
			})
		}
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
				brk_ptr: self.state.brk_ptr,
			},
			vmem: new_vmem,
		})
	}

	/// Allocates the physical pages on the given range.
	///
	/// Arguments:
	/// - `virtaddr` is the virtual address to beginning of the range to allocate.
	/// - `len` is the size of the range in bytes.
	///
	/// If the mapping doesn't exist, the function returns an error.
	///
	/// On error, allocations that have been made are not freed as it does not affect the behaviour
	/// from the user's point of view.
	pub fn alloc(&mut self, virtaddr: *const c_void, len: usize) -> AllocResult<()> {
		let mut transaction = self.vmem.transaction();
		let mut off = 0;
		while off < len {
			let virtaddr = (virtaddr as usize + off) as *const c_void;
			if let Some(mapping) = self.state.get_mut_mapping_for_ptr(virtaddr) {
				let page_offset =
					(virtaddr as usize - mapping.get_begin() as usize) / memory::PAGE_SIZE;
				mapping.alloc(page_offset, &mut transaction)?;
			}
			off += memory::PAGE_SIZE;
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

	/// Returns the pointer for the `brk` syscall.
	pub fn get_brk_ptr(&self) -> *mut c_void {
		self.state.brk_ptr
	}

	/// Sets the initial pointer for the `brk` syscall.
	///
	/// This function MUST be called *only once*, before the program starts.
	///
	/// `ptr` MUST be page-aligned.
	pub fn set_brk_init(&mut self, ptr: *mut c_void) {
		debug_assert!(ptr.is_aligned_to(memory::PAGE_SIZE));
		self.state.brk_init = ptr;
		self.state.brk_ptr = ptr;
	}

	/// Sets the pointer for the `brk` syscall.
	///
	/// If the memory cannot be allocated, the function returns an error.
	#[allow(clippy::not_unsafe_ptr_arg_deref)]
	pub fn set_brk_ptr(&mut self, ptr: *mut c_void) -> AllocResult<()> {
		if ptr >= self.state.brk_ptr {
			// Check the pointer is valid
			if ptr > COPY_BUFFER as _ {
				return Err(AllocError);
			}
			// Allocate memory
			let begin = unsafe { utils::align(self.state.brk_ptr, memory::PAGE_SIZE) };
			let pages = (ptr as usize - begin as usize).div_ceil(memory::PAGE_SIZE);
			let Some(pages) = NonZeroUsize::new(pages) else {
				return Ok(());
			};
			let flags = MAPPING_FLAG_WRITE | MAPPING_FLAG_USER;
			self.map(
				MapConstraint::Fixed(begin as _),
				pages,
				flags,
				MapResidence::Normal,
			)?;
		} else {
			// Check the pointer is valid
			if ptr < self.state.brk_init {
				return Err(AllocError);
			}
			// Free memory
			let begin = unsafe { utils::align(ptr, memory::PAGE_SIZE) };
			let pages = (begin as usize - ptr as usize).div_ceil(memory::PAGE_SIZE);
			let Some(pages) = NonZeroUsize::new(pages) else {
				return Ok(());
			};
			self.unmap(begin, pages, true)?;
		}
		self.state.brk_ptr = ptr;
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
	/// - `virtaddr` is the virtual address of the wrong memory access that caused the fault.
	/// - `code` is the error code given along with the error.
	///
	/// If the process should continue, the function returns `true`, else `false`.
	pub fn handle_page_fault(&mut self, virtaddr: *const c_void, code: u32) -> bool {
		if code & vmem::x86::PAGE_FAULT_PRESENT == 0 {
			return false;
		}
		let Some(mapping) = self.state.get_mut_mapping_for_ptr(virtaddr) else {
			return false;
		};
		// Check permissions
		let code_write = code & vmem::x86::PAGE_FAULT_WRITE != 0;
		let mapping_write = mapping.get_flags() & MAPPING_FLAG_WRITE != 0;
		if code_write && !mapping_write {
			return false;
		}
		// TODO check exec
		let code_userspace = code & vmem::x86::PAGE_FAULT_USER != 0;
		let mapping_userspace = mapping.get_flags() & MAPPING_FLAG_USER != 0;
		if code_userspace && !mapping_userspace {
			return false;
		}
		// Map the accessed page
		let page_offset = (virtaddr as usize - mapping.get_begin() as usize) / memory::PAGE_SIZE;
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
	use core::ptr::null;

	#[test_case]
	fn test0() {
		let mut mem_space = MemSpace::new().unwrap();
		let addr = 0x1000 as _;
		let size = NonZeroUsize::new(1).unwrap();
		let res = mem_space
			.map(
				MapConstraint::Fixed(addr),
				size,
				MAPPING_FLAG_WRITE | MAPPING_FLAG_USER,
				MapResidence::Normal,
			)
			.unwrap();
		assert_eq!(res, addr);
		assert!(!mem_space.can_access(null(), memory::PAGE_SIZE, true, true));
		assert!(!mem_space.can_access(null(), memory::PAGE_SIZE + 1, true, true));
		assert!(mem_space.can_access(addr as _, memory::PAGE_SIZE, true, true));
		assert!(!mem_space.can_access(addr as _, memory::PAGE_SIZE + 1, true, true));
		mem_space.unmap(addr, size, false).unwrap();
		assert!(!mem_space.can_access(addr as _, memory::PAGE_SIZE, true, true));
	}
}
