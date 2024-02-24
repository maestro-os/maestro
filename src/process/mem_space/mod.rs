//! A memory space is a virtual memory handler for a process. It handles virtual and physical
//! memory allocations for the process, as well as linkage between them.
//!
//! The memory space contains two types of structures:
//! - Mapping: A chunk of virtual memory that is allocated
//! - Gap: A chunk of virtual memory that is available to be allocated

mod gap;
mod mapping;
pub mod ptr;
mod transaction;

use crate::{
	errno::{AllocError, Errno},
	file::{perm::AccessProfile, FileLocation},
	idt, memory,
	memory::{buddy, physical_ref_counter::PhysRefCounter, stack, vmem, vmem::VMem},
	process::{oom, open_file::OpenFile, AllocResult},
	util,
	util::{
		collections::{
			btreemap::{BTreeMap, Entry},
			vec::Vec,
		},
		lock::Mutex,
		ptr::arc::Arc,
		TryClone,
	},
};
use core::{
	cmp::{min, Ordering},
	ffi::c_void,
	fmt,
	mem::size_of,
	num::NonZeroUsize,
	ptr::{null_mut, NonNull},
};
use gap::MemGap;
use mapping::MemMapping;
use transaction::MemSpaceTransaction;

/// Flag telling that a memory mapping can be written to.
pub const MAPPING_FLAG_WRITE: u8 = 0b00001;
/// Flag telling that a memory mapping can contain executable instructions.
pub const MAPPING_FLAG_EXEC: u8 = 0b00010;
/// Flag telling that a memory mapping is accessible from userspace.
pub const MAPPING_FLAG_USER: u8 = 0b00100;
/// Flag telling that a memory mapping must allocate its physical memory right
/// away and not when the process tries to write to it.
pub const MAPPING_FLAG_NOLAZY: u8 = 0b01000;
/// Flag telling that a memory mapping has its physical memory shared with one
/// or more other mappings.
///
/// If the mapping is associated with a file, modifications made to the mapping are update to the
/// file.
pub const MAPPING_FLAG_SHARED: u8 = 0b10000;

/// The physical pages reference counter.
pub static PHYSICAL_REF_COUNTER: Mutex<PhysRefCounter> = Mutex::new(PhysRefCounter::new());

// TODO when reaching the last reference to the open file, close it on unmap

// TODO Disallow clone and use a special function + Drop to increment/decrement reference counters
/// A map residence is the location to which the data on the physical memory of a mapping is to be
/// synchronized.
#[derive(Clone)]
pub enum MapResidence {
	/// The mapping does not reside anywhere except on the main memory.
	Normal,

	/// The mapping points to a static location, which may or may not be shared between several
	/// memory spaces.
	Static {
		/// The list of memory pages, in order.
		pages: Arc<Vec<NonNull<[u8; memory::PAGE_SIZE]>>>,
	},

	/// The mapping resides in a file.
	File {
		/// The location of the file.
		location: FileLocation,
		/// The offset of the mapping in the file.
		off: u64,
	},

	/// The mapping resides in swap space.
	Swap {
		/// The location of the swap space.
		swap_file: Arc<Mutex<OpenFile>>,
		/// The ID of the slot occupied by the mapping.
		slot_id: u32,
		/// The page offset in the slot.
		page_off: usize,
	},
}

impl MapResidence {
	/// Tells whether the residence is normal.
	pub fn is_normal(&self) -> bool {
		matches!(self, MapResidence::Normal)
	}

	/// Adds a value of `pages` pages to the offset of the residence, if applicable.
	pub fn offset_add(&mut self, pages: usize) {
		match self {
			Self::File {
				off, ..
			} => *off += pages as u64 * memory::PAGE_SIZE as u64,

			Self::Swap {
				page_off, ..
			} => *page_off += pages,

			_ => {}
		}
	}

	/// TODO doc
	fn alloc() -> AllocResult<NonNull<c_void>> {
		let ptr = buddy::alloc(0, buddy::FLAG_ZONE_TYPE_USER)?;
		let mut ref_counter = PHYSICAL_REF_COUNTER.lock();
		match ref_counter.increment(ptr.as_ptr()) {
			Ok(()) => Ok(ptr),
			Err(e) => {
				unsafe {
					buddy::free(ptr.as_ptr(), 0);
				}
				Err(e)
			}
		}
	}

	/// TODO doc
	fn free(ptr: *const c_void) {
		let mut ref_counter = PHYSICAL_REF_COUNTER.lock();
		ref_counter.decrement(ptr);
		if ref_counter.can_free(ptr) {
			unsafe {
				buddy::free(ptr, 0);
			}
		}
	}

	/// Allocates a physical page for the given offset.
	///
	/// Since the function might reuse the same page for several allocation, the page must be freed
	/// only using the `free_page` function associated with the current instance.
	pub fn alloc_page(&self, off: usize) -> AllocResult<NonNull<c_void>> {
		match self {
			MapResidence::Normal => Self::alloc(),

			MapResidence::Static {
				pages,
			} => {
				if off < pages.len() {
					Ok(pages[off].cast())
				} else {
					Self::alloc()
				}
			}

			MapResidence::File {
				location: _,
				off: _,
			} => {
				// TODO get physical page for this offset
				todo!();
			}

			MapResidence::Swap {
				..
			} => {
				// TODO
				todo!();
			}
		}
	}

	/// Frees the page allocated with `alloc_page`.
	pub fn free_page(&self, off: usize, ptr: *const c_void) {
		match self {
			MapResidence::Normal => Self::free(ptr),

			MapResidence::Static {
				pages,
			} => {
				if off >= pages.len() {
					Self::free(ptr)
				}
			}

			MapResidence::File {
				location: _,
				off: _,
			} => {
				// TODO
				todo!();
			}

			MapResidence::Swap {
				..
			} => {
				// TODO
				todo!();
			}
		}
	}
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
			// Checking the address is within userspace is required because Fixed allocation can
			// take place *outside of gaps* but *not inside the kernelspace*
			MapConstraint::Fixed(addr) => {
				*addr <= memory::PROCESS_END && addr.is_aligned_to(memory::PAGE_SIZE)
			}
			MapConstraint::Hint(addr) => addr.is_aligned_to(memory::PAGE_SIZE),
			_ => true,
		}
	}
}

/// The set of mapped regions and free gaps of a memory space.
///
/// Separation is necessary to allow rollback-able transactions in case an operation fails.
/// This is done by creating a fresh instance, then merging when fallible operations succeed.
#[derive(Default)]
struct MemSpaceState {
	/// Binary tree storing the list of memory gaps, ready for new mappings.
	///
	/// The collection is sorted by pointer to the beginning of the mapping on the virtual
	/// memory.
	gaps: BTreeMap<*mut c_void, MemGap>,
	/// Binary tree storing the list of memory gaps, sorted by size and then by
	/// beginning address.
	gaps_size: BTreeMap<(NonZeroUsize, *mut c_void), ()>,
	/// Binary tree storing the list of memory mappings.
	///
	/// Sorted by pointer to the beginning of the mapping on the virtual memory.
	mappings: BTreeMap<*mut c_void, MemMapping>,
}

impl MemSpaceState {
	/// Inserts the given gap into the state.
	fn insert_gap(&mut self, gap: MemGap) -> AllocResult<()> {
		let gap_ptr = gap.get_begin();
		let gap_size = gap.get_size();
		self.gaps.insert(gap_ptr, gap)?;
		if let Err(e) = self.gaps_size.insert((gap_size, gap_ptr), ()) {
			// On allocation error, rollback
			self.gaps.remove(&gap_ptr);
			return Err(e);
		}
		Ok(())
	}

	/// Removes the gap beginning at the given address from the state.
	///
	/// The function returns the removed gap.
	///
	/// If the gap didn't exist, the function returns `None`.
	fn remove_gap(&mut self, gap_begin: *mut c_void) -> Option<MemGap> {
		let g = self.gaps.remove(&gap_begin)?;
		self.gaps_size.remove(&(g.get_size(), gap_begin));
		Some(g)
	}

	/// Returns a reference to a gap with at least size `size`.
	///
	/// `size` is the minimum size of the gap to be returned.
	///
	/// If no gap large enough is available, the function returns `None`.
	fn get_gap(&self, size: NonZeroUsize) -> Option<&MemGap> {
		let ((_, ptr), _) = self
			.gaps_size
			.range((size, null_mut::<c_void>())..)
			.next()?;
		let gap = self.gaps.get(ptr).unwrap();
		debug_assert!(gap.get_size() >= size);
		Some(gap)
	}

	/// Returns a reference to the gap containing the given virtual address `ptr`.
	///
	/// If no gap contain the pointer, the function returns `None`.
	fn get_gap_for_ptr(&self, ptr: *const c_void) -> Option<&MemGap> {
		self.gaps.cmp_get(|key, value| {
			let begin = *key as usize;
			let end = begin + (value.get_size().get() * memory::PAGE_SIZE);
			if (ptr as usize) >= begin && (ptr as usize) < end {
				Ordering::Equal
			} else if (ptr as usize) < begin {
				Ordering::Less
			} else {
				Ordering::Greater
			}
		})
	}

	/// Returns an immutable reference to the memory mapping containing the given virtual
	/// address `ptr`.
	///
	/// If no mapping contains the address, the function returns `None`.
	fn get_mapping_for_ptr(&self, ptr: *const c_void) -> Option<&MemMapping> {
		self.mappings.cmp_get(|key, value| {
			let begin = *key as usize;
			let end = begin + (value.get_size().get() * memory::PAGE_SIZE);
			if (ptr as usize) >= begin && (ptr as usize) < end {
				Ordering::Equal
			} else if (ptr as usize) < begin {
				Ordering::Less
			} else {
				Ordering::Greater
			}
		})
	}
}

/// A virtual memory space.
pub struct MemSpace {
	/// The state of the memory space's mapped regions and free gaps.
	state: MemSpaceState,

	/// The number of used virtual memory pages.
	vmem_usage: usize,

	/// The initial pointer of the `[s]brk` system calls.
	brk_init: *mut c_void,
	/// The current pointer of the `[s]brk` system calls.
	brk_ptr: *mut c_void,

	/// Architecture-specific virtual memory context handler.
	vmem: VMem,
}

impl MemSpace {
	/// Creates a new virtual memory object.
	pub fn new() -> AllocResult<Self> {
		let mut s = Self {
			state: Default::default(),

			vmem_usage: 0,

			brk_init: null_mut::<_>(),
			brk_ptr: null_mut::<_>(),

			vmem: VMem::new()?,
		};
		// Create the default gap of memory which is present at the beginning
		let begin = memory::ALLOC_BEGIN;
		let size = (memory::PROCESS_END as usize - begin as usize) / memory::PAGE_SIZE;
		let gap = MemGap::new(begin, NonZeroUsize::new(size).unwrap());
		s.state.insert_gap(gap)?;
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
		self.vmem_usage
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
	/// the memory is detected, unless [`MAPPING_FLAG_NOLAZY`] is specified as a flag.
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
		let mut transaction = MemSpaceTransaction::default();
		let mut vmem_usage = self.vmem_usage;
		// Get gap suitable for the given constraint
		let (gap, off) = match map_constraint {
			MapConstraint::Fixed(addr) => {
				vmem_usage -= self.unmap_impl(&mut transaction, addr, size, false)?;
				// FIXME: unmapping might create gaps which need to be removed, or else it will be
				// possible to clobber the mapping to be created in the current operation
				// Create a fictive gap. This is required because fixed allocations may be used
				// outside allowed gaps
				let gap = MemGap {
					begin: addr,
					size,
				};
				(gap, 0)
			}
			MapConstraint::Hint(addr) => {
				// Get the gap for the pointer
				let gap = self.state.get_gap_for_ptr(addr).ok_or(AllocError)?.clone();
				let off = gap.get_page_offset_for(addr);
				// Check whether the mapping fits in the gap
				let fit = off
					.checked_add(size.get())
					.map(|end| end <= gap.get_size().get())
					.unwrap_or(false);
				if fit {
					(gap, off)
				} else {
					// Hint cannot be satisfied. Get a gap large enough
					let gap = self.state.get_gap(size).ok_or(AllocError)?.clone();
					(gap, 0)
				}
			}
			MapConstraint::None => {
				let gap = self.state.get_gap(size).ok_or(AllocError)?.clone();
				(gap, 0)
			}
		};
		let addr = unsafe { gap.get_begin().add(off * memory::PAGE_SIZE) };
		// Split the old gap to fit the mapping, and insert new gaps
		let (left_gap, right_gap) = gap.consume(off, size.get());
		self.state.remove_gap(gap.get_begin());
		if let Some(new_gap) = left_gap {
			transaction.buffer_state.insert_gap(new_gap)?;
		}
		if let Some(new_gap) = right_gap {
			transaction.buffer_state.insert_gap(new_gap)?;
		}
		// Create the mapping
		let mapping = MemMapping::new(addr, size, flags, residence);
		vmem_usage += size.get();
		let m = match transaction.buffer_state.mappings.entry(addr) {
			Entry::Vacant(e) => e.insert(mapping)?,
			// Occupied means the state is inconsistent
			Entry::Occupied(_) => unreachable!(),
		};
		transaction.commit(self)?;
		self.vmem_usage = vmem_usage;
		Ok(addr)
	}

	/// Returns an immutable reference to the memory mapping containing the given
	/// virtual address `ptr`.
	///
	/// If no mapping contains the address, the function returns `None`.
	pub fn get_mapping_for_ptr(&self, ptr: *const c_void) -> Option<&MemMapping> {
		self.state.get_mapping_for_ptr(ptr)
	}

	// TODO Optimize (currently O(n log n))
	/// Implementation for `unmap`.
	///
	/// The function returns the number of pages freed.
	fn unmap_impl(
		&mut self,
		transaction: &mut MemSpaceTransaction,
		ptr: *const c_void,
		size: NonZeroUsize,
		brk: bool,
	) -> AllocResult<usize> {
		let mut freed = 0;
		// Remove every mapping in the chunk to unmap
		let mut i = 0;
		while i < size.get() {
			// The current page's beginning
			let page_ptr = unsafe { ptr.add(i * memory::PAGE_SIZE) };
			// The mapping containing the page
			let Some(mapping) = self.state.get_mapping_for_ptr(page_ptr) else {
				// TODO jump to next mapping directly using binary tree
				i += 1;
				continue;
			};
			// The pointer to the beginning of the mapping
			let mapping_ptr = mapping.get_begin();
			transaction.remove_mappings.push(mapping_ptr)?;
			// The offset in the mapping to the beginning of pages to unmap
			let begin = (page_ptr as usize - mapping_ptr as usize) / memory::PAGE_SIZE;
			// The number of pages to unmap in the mapping
			let pages = min(size.get() - i, mapping.get_size().get() - begin);
			i += pages;
			// Newly created mappings and gap after removing parts of the previous one
			let (prev, gap, next) = mapping.split(begin, pages);
			// Insert new mappings
			if let Some(p) = prev {
				transaction.buffer_state.mappings.insert(p.get_begin(), p)?;
			}
			if let Some(n) = next {
				transaction.buffer_state.mappings.insert(n.get_begin(), n)?;
			}
			// Do not create gaps if unmapping for `*brk` system calls as this space is reserved by
			// it and must not be reused by `mmap`
			if brk {
				continue;
			}
			// Insert gap
			if let Some(mut gap) = gap {
				freed += gap.get_size().get();
				// Merge previous gap
				let prev_gap = (!gap.get_begin().is_null())
					.then(|| {
						let prev_gap_ptr = unsafe { gap.get_begin().sub(1) };
						self.state.get_gap_for_ptr(prev_gap_ptr)
					})
					.flatten();
				if let Some(p) = prev_gap {
					transaction.remove_gaps.push(p.get_begin())?;
					gap.merge(p);
				}
				// Merge next gap
				let next_gap = self.state.get_gap_for_ptr(gap.get_end());
				if let Some(n) = next_gap {
					transaction.remove_gaps.push(n.get_begin())?;
					gap.merge(n);
				}
				transaction.buffer_state.insert_gap(gap)?;
			}
		}
		Ok(freed)
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
		let mut transaction = MemSpaceTransaction::default();
		let removed_count = self.unmap_impl(&mut transaction, ptr, size, brk)?;
		transaction.commit(self)?;
		self.vmem_usage -= removed_count;
		Ok(())
	}

	/// Same as `map`, except the function returns a pointer to the end of the
	/// memory mapping.
	pub fn map_stack(&mut self, size: NonZeroUsize, flags: u8) -> AllocResult<*mut c_void> {
		let mapping_ptr = self.map(MapConstraint::None, size, flags, MapResidence::Normal)?;
		Ok(unsafe {
			// Safe because the new pointer stays in the range of the allocated mapping
			mapping_ptr.add(size.get() * memory::PAGE_SIZE)
		})
	}

	/// Same as `unmap`, except the function takes a pointer to the end of the
	/// memory mapping.
	#[allow(clippy::not_unsafe_ptr_arg_deref)]
	pub fn unmap_stack(&mut self, ptr: *const c_void, size: NonZeroUsize) -> AllocResult<()> {
		// Safe because the new pointer stays in the range of the allocated mapping
		let ptr = unsafe { ptr.sub(size.get() * memory::PAGE_SIZE) };
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
					let page_begin = util::down_align(curr_ptr as _, memory::PAGE_SIZE);
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

	/// Binds the memory space to the current core.
	pub fn bind(&self) {
		self.vmem.bind();
	}

	/// Tells whether the memory space is bound.
	pub fn is_bound(&self) -> bool {
		self.vmem.is_bound()
	}

	/// Performs the fork operation.
	fn do_fork(&mut self) -> AllocResult<Self> {
		let mut mem_space = Self {
			state: MemSpaceState {
				gaps: self.state.gaps.try_clone()?,
				gaps_size: self.state.gaps_size.try_clone()?,
				mappings: BTreeMap::new(),
			},

			vmem_usage: self.vmem_usage,

			brk_init: self.brk_init,
			brk_ptr: self.brk_ptr,

			vmem: self.vmem.try_clone()?,
		};
		for (_, m) in self.state.mappings.iter_mut() {
			let new_mapping = m.fork(&mut mem_space.vmem)?;
			for i in 0..new_mapping.get_size().get() {
				m.update_vmem(i, &mut mem_space.vmem);
				new_mapping.update_vmem(i, &mut mem_space.vmem);
			}
			mem_space
				.state
				.mappings
				.insert(new_mapping.get_begin(), new_mapping)?;
		}
		Ok(mem_space)
	}

	/// Clones the current memory space for process forking.
	pub fn fork(&mut self) -> AllocResult<MemSpace> {
		idt::wrap_disable_interrupts(|| unsafe { stack::switch(None, || self.do_fork()) })?
	}

	/// Allocates the physical pages to write on the given pointer.
	///
	/// `virtaddr` is the address to allocate.
	///
	/// The size of the memory chunk to allocated equals `size_of::<T>() * len`.
	///
	/// If the mapping doesn't exist, the function returns an error.
	pub fn alloc<T>(&mut self, virtaddr: *const T, len: usize) -> AllocResult<()> {
		let mut off = 0;
		while off < size_of::<T>() * len {
			let virtaddr = unsafe { (virtaddr as *const c_void).add(off) };
			if let Some(mapping) = self.state.get_mapping_for_ptr(virtaddr) {
				let page_offset =
					(virtaddr as usize - mapping.get_begin() as usize) / memory::PAGE_SIZE;
				oom::wrap(|| mapping.alloc(page_offset, &mut self.vmem));
				mapping.update_vmem(page_offset, &mut self.vmem);
			}
			off += memory::PAGE_SIZE;
		}
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
	) -> Result<(), Errno> {
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
		self.brk_ptr
	}

	/// Sets the initial pointer for the `brk` syscall.
	///
	/// This function MUST be called *only once*, before the program starts.
	///
	/// `ptr` MUST be page-aligned.
	pub fn set_brk_init(&mut self, ptr: *mut c_void) {
		debug_assert!(ptr.is_aligned_to(memory::PAGE_SIZE));
		self.brk_init = ptr;
		self.brk_ptr = ptr;
	}

	/// Sets the pointer for the `brk` syscall.
	///
	/// If the memory cannot be allocated, the function returns an error.
	#[allow(clippy::not_unsafe_ptr_arg_deref)]
	pub fn set_brk_ptr(&mut self, ptr: *mut c_void) -> AllocResult<()> {
		if ptr >= self.brk_ptr {
			// Check the pointer is valid
			if ptr > memory::PROCESS_END {
				return Err(AllocError);
			}
			// Allocate memory
			let begin = unsafe { util::align(self.brk_ptr, memory::PAGE_SIZE) };
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
			if ptr < self.brk_init {
				return Err(AllocError);
			}
			// Free memory
			let begin = unsafe { util::align(ptr, memory::PAGE_SIZE) };
			let pages = (begin as usize - ptr as usize).div_ceil(memory::PAGE_SIZE);
			let Some(pages) = NonZeroUsize::new(pages) else {
				return Ok(());
			};
			self.unmap(begin, pages, true)?;
		}
		self.brk_ptr = ptr;
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
		let Some(mapping) = self.state.get_mapping_for_ptr(virtaddr) else {
			return false;
		};
		// Check permissions
		let can_write_mapping = mapping.get_flags() & MAPPING_FLAG_WRITE != 0;
		if code & vmem::x86::PAGE_FAULT_WRITE != 0 && !can_write_mapping {
			return false;
		}
		// TODO check exec
		let userspace_mapping = mapping.get_flags() & MAPPING_FLAG_USER != 0;
		if code & vmem::x86::PAGE_FAULT_USER != 0 && !userspace_mapping {
			return false;
		}
		// Map the accessed page
		let page_offset = (virtaddr as usize - mapping.get_begin() as usize) / memory::PAGE_SIZE;
		oom::wrap(|| mapping.alloc(page_offset, &mut self.vmem));
		mapping.update_vmem(page_offset, &mut self.vmem);
		true
	}
}

impl fmt::Debug for MemSpace {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{{mappings: [")?;
		for (i, (_, m)) in self.state.mappings.iter().enumerate() {
			if i + 1 < self.state.mappings.len() {
				write!(f, "{m:?}, ")?;
			} else {
				write!(f, "{m:?}")?;
			}
		}
		write!(f, "], gaps: [")?;
		for (i, (_, g)) in self.state.gaps.iter().enumerate() {
			if i + 1 < self.state.gaps.len() {
				write!(f, "{g:?}, ")?;
			} else {
				write!(f, "{g:?}")?;
			}
		}
		write!(f, "]}}")
	}
}

impl Drop for MemSpace {
	fn drop(&mut self) {
		// Unmapping virtual pages is done by dropping `self.vmem`
		// Unmap all physical pages
		for (_, mapping) in &mut self.state.mappings {
			for off in 0..mapping.get_size().get() {
				unsafe {
					mapping.free_phys_page(off, &self.vmem);
				}
			}
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
