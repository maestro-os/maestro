//! This module implements the MemSpace structure which is responsible for
//! handling the memory mapping of execution contexts.
//!
//! The memory space contains two types of structures:
//! - Mapping: A chunk of virtual memory that is allocated
//! - Gap: A chunk of virtual memory that is available to be allocated

mod gap;
mod mapping;
pub mod ptr;

use core::cmp::Ordering;
use core::cmp::min;
use core::ffi::c_void;
use core::fmt;
use core::mem::size_of;
use core::ptr::NonNull;
use core::ptr::null;
use crate::errno::Errno;
use crate::errno;
use crate::file::open_file::OpenFile;
use crate::idt;
use crate::memory::physical_ref_counter::PhysRefCounter;
use crate::memory::stack;
use crate::memory::vmem::VMem;
use crate::memory::vmem;
use crate::memory;
use crate::process::oom;
use crate::util::FailableClone;
use crate::util::boxed::Box;
use crate::util::container::map::Map;
use crate::util::lock::Mutex;
use crate::util::math;
use crate::util::ptr::SharedPtr;
use crate::util;
use gap::MemGap;
use mapping::MemMapping;

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

/// Enumeration of constraints for memory mapping.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MapConstraint {
	/// The mapping is done at a fixed address. Previous allocations at the same
	/// place are unmapped.
	Fixed(*const c_void),

	/// The mapping is done at a fixed address. If the address range is already
	/// in use, the allocation fails.
	Hint(*const c_void),

	/// No constraint.
	None,
}

/// Structure representing the virtual memory space of a context.
pub struct MemSpace {
	/// Binary tree storing the list of memory gaps, ready for new mappings.
	/// Sorted by pointer to the beginning of the mapping on the virtual memory.
	gaps: Map<*const c_void, MemGap>,
	/// Binary tree storing the list of memory gaps, sorted by size and then by
	/// beginning address.
	gaps_size: Map<(usize, *const c_void), ()>,

	/// Binary tree storing the list of memory mappings. Sorted by pointer to
	/// the beginning of the mapping on the virtual memory.
	mappings: Map<*const c_void, MemMapping>,

	/// The number of used virtual memory pages.
	vmem_usage: usize,

	/// The initial pointer of the `brk` system call.
	brk_init: *const c_void,
	/// The current pointer of the `brk` system call.
	brk_ptr: *const c_void,

	/// The virtual memory context handler.
	vmem: Box<dyn VMem>,
}

impl MemSpace {
	/// Inserts the given gap into the memory space's structures.
	fn gap_insert(&mut self, gap: MemGap) -> Result<(), Errno> {
		let gap_ptr = gap.get_begin();
		let g = self.gaps.insert(gap_ptr, gap)?;

		if let Err(e) = self.gaps_size.insert((g.get_size(), gap_ptr), ()) {
			self.gaps.remove(gap_ptr);
			return Err(e);
		}

		Ok(())
	}

	/// Removes the given gap from the memory space's structures.
	/// The function returns the removed gap. If the gap didn't exist, the
	/// function returns None.
	fn gap_remove(&mut self, gap_begin: *const c_void) -> Option<MemGap> {
		let g = self.gaps.remove(gap_begin)?;
		self.gaps_size.remove((g.get_size(), gap_begin));

		Some(g)
	}

	/// Returns a reference to a gap with at least size `size`.
	/// `gaps` is the binary tree storing gaps, sorted by pointer to their
	/// respective beginnings. `gaps_size` is the binary tree storing pointers
	/// to gaps, sorted by gap sizes. `size` is the minimum size of the gap.
	/// If no gap large enough is available, the function returns None.
	fn gap_get<'a>(
		gaps: &'a Map<*const c_void, MemGap>,
		gaps_size: &Map<(usize, *const c_void), ()>,
		size: usize,
	) -> Option<&'a MemGap> {
		let (_, ptr) = gaps_size.get_min((size, 0 as _))?.0;
		let gap = gaps.get(*ptr).unwrap();
		debug_assert!(gap.get_size() >= size);

		Some(gap)
	}

	/// Returns a reference to the gap containing the pointer `ptr`.
	/// `gaps` is the binary tree storing gaps, sorted by pointer to their
	/// respective beginnings. `ptr` is the pointer.
	/// If no gap contain the pointer, the function returns None.
	fn gap_by_ptr<'a>(
		gaps: &'a Map<*const c_void, MemGap>,
		ptr: *const c_void,
	) -> Option<&'a MemGap> {
		gaps.cmp_get(|key, value| {
			let begin = *key;
			let end = (begin as usize + value.get_size() * memory::PAGE_SIZE) as *const c_void;

			if ptr >= begin && ptr < end {
				Ordering::Equal
			} else if ptr < begin {
				Ordering::Less
			} else {
				Ordering::Greater
			}
		})
	}

	/// Returns a new binary tree containing the default gaps for a memory
	/// space.
	fn create_default_gaps(&mut self) -> Result<(), Errno> {
		let begin = memory::ALLOC_BEGIN;
		let size = (memory::PROCESS_END as usize - begin as usize) / memory::PAGE_SIZE;
		self.gap_insert(MemGap::new(begin, size))
	}

	/// Clones the `gaps_size` field.
	fn gaps_size_clone(&self) -> Result<Map<(usize, *const c_void), ()>, Errno> {
		let mut gaps_size = Map::new();
		for (g, _) in &self.gaps_size {
			gaps_size.insert(g.clone(), ())?;
		}

		Ok(gaps_size)
	}

	/// Creates a new virtual memory object.
	/// `brk_ptr` is the initial pointer for the `brk` syscall.
	pub fn new() -> Result<Self, Errno> {
		let mut s = Self {
			gaps: Map::new(),
			gaps_size: Map::new(),

			mappings: Map::new(),

			vmem_usage: 0,

			brk_init: null::<_>(),
			brk_ptr: null::<_>(),

			vmem: vmem::new()?,
		};
		s.create_default_gaps()?;
		Ok(s)
	}

	/// Returns a mutable reference to the vvirtual memory context.
	pub fn get_vmem(&mut self) -> &mut Box<dyn VMem> {
		&mut self.vmem
	}

	/// Returns the number of virtual memory pages in the memory space.
	pub fn get_vmem_usage(&self) -> usize {
		self.vmem_usage
	}

	// TODO Fix potential invalid state on fail
	/// Maps a chunk of memory.
	///
	/// The function has complexity `O(log n)`.
	///
	/// Arguments:
	/// - `map_constraint` is the constraint to fullfill for the allocation.
	/// - `size` represents the size of the mapping in number of memory pages.
	/// - `flags` represents the flags for the mapping.
	/// - `file` is the open file to map to, along with an offset in this file.
	///
	/// The underlying physical memory is not allocated directly but only when an attempt to write
	/// the memory is detected, unless MAPPING_FLAG_NOLAZY is specified as a flag.
	///
	/// On success, the function returns a pointer to the newly mapped virtual memory.
	///
	/// If the given pointer is not page-aligned, the function returns an error.
	pub fn map(
		&mut self,
		map_constraint: MapConstraint,
		size: usize,
		flags: u8,
		file: Option<(SharedPtr<OpenFile>, u64)>,
	) -> Result<*mut c_void, Errno> {
		// Checking arguments are valid
		match map_constraint {
			MapConstraint::Fixed(ptr) | MapConstraint::Hint(ptr) => {
				if !util::is_aligned(ptr, memory::PAGE_SIZE) {
					return Err(errno!(EINVAL));
				}
			}

			_ => {}
		}
		if size == 0 {
			return Err(errno!(EINVAL));
		}

		// Mapping informations matching mapping constraints
		let (gap, addr) = match map_constraint {
			MapConstraint::Fixed(addr) => {
				self.unmap(addr, size, false)?;
				let gap = Self::gap_by_ptr(&self.gaps, addr);

				(gap, addr as _)
			}

			MapConstraint::Hint(addr) => {
				// Getting the gap for the pointer
				let mut gap = Self::gap_by_ptr(&self.gaps, addr)
					.ok_or_else(|| errno!(ENOMEM))?;

				// The offset in the gap
				let off = (addr as usize - gap.get_begin() as usize) / memory::PAGE_SIZE;
				if off + size > gap.get_size() {
					// Hint cannot be satisfied. Get a gap large enough
					gap = Self::gap_get(&self.gaps, &self.gaps_size, size)
						.ok_or_else(|| errno!(ENOMEM))?;
				}

				let addr = unsafe {
					gap.get_begin().add(off * memory::PAGE_SIZE)
				};
				(Some(gap), addr)
			}

			MapConstraint::None => {
				let gap = Self::gap_get(&self.gaps, &self.gaps_size, size)
					.ok_or_else(|| errno!(ENOMEM))?;
				(Some(gap), gap.get_begin())
			}
		};

		// Creating the mapping
		let mapping = MemMapping::new(
			addr,
			size,
			flags,
			file,
			NonNull::new(self.vmem.as_mut_ptr()).unwrap(),
		);
		let m = self.mappings.insert(addr, mapping)?;

		// Mapping the default page
		if let Err(e) = m.map_default() {
			self.mappings.remove(addr);
			return Err(e);
		}

		// Splitting the old gap to fit the mapping if needed
		if let Some(gap) = gap {
			let off = (addr as usize - gap.get_begin() as usize) / memory::PAGE_SIZE;
			let (left_gap, right_gap) = gap.consume(off, size);

			// Removing the old gap
			let gap_begin = gap.get_begin();
			self.gap_remove(gap_begin);

			// Inserting the new gaps
			if let Some(new_gap) = left_gap {
				oom::wrap(|| self.gap_insert(new_gap.clone()));
			}
			if let Some(new_gap) = right_gap {
				oom::wrap(|| self.gap_insert(new_gap.clone()));
			}
		}

		self.vmem_usage += size;
		Ok(addr as *mut _)
	}

	/// Same as `map`, except the function returns a pointer to the end of the
	/// memory mapping.
	pub fn map_stack(&mut self, size: usize, flags: u8) -> Result<*mut c_void, Errno> {
		let mapping_ptr = self.map(MapConstraint::None, size, flags, None)?;

		Ok(unsafe {
			// Safe because the new pointer stays in the range of the allocated mapping
			mapping_ptr.add(size * memory::PAGE_SIZE)
		})
	}

	/// Same as `unmap`, except the function takes a pointer to the end of the
	/// memory mapping.
	pub fn unmap_stack(&mut self, ptr: *const c_void, size: usize) -> Result<(), Errno> {
		// Safe because the new pointer stays in the range of the allocated mapping
		let ptr = unsafe { ptr.sub(size * memory::PAGE_SIZE) };

		self.unmap(ptr, size, false)
	}

	/// Returns a reference to the memory mapping containing the given virtual
	/// address `ptr` from mappings container `mappings`. If no mapping contains
	/// the address, the function returns None.
	fn get_mapping_for_(
		mappings: &Map<*const c_void, MemMapping>,
		ptr: *const c_void,
	) -> Option<&MemMapping> {
		mappings.cmp_get(|key, value| {
			let begin = *key;
			let end = (begin as usize + value.get_size() * memory::PAGE_SIZE) as *const c_void;

			if ptr >= begin && ptr < end {
				Ordering::Equal
			} else if ptr < begin {
				Ordering::Less
			} else {
				Ordering::Greater
			}
		})
	}

	/// Returns a mutable reference to the memory mapping containing the given
	/// virtual address `ptr` from mappings container `mappings`. If no mapping
	/// contains the address, the function returns None.
	fn get_mapping_mut_for_(
		mappings: &mut Map<*const c_void, MemMapping>,
		ptr: *const c_void,
	) -> Option<&mut MemMapping> {
		mappings.cmp_get_mut(|key, value| {
			let begin = *key;
			let end = (begin as usize + value.get_size() * memory::PAGE_SIZE) as *const c_void;

			if ptr >= begin && ptr < end {
				Ordering::Equal
			} else if ptr < begin {
				Ordering::Less
			} else {
				Ordering::Greater
			}
		})
	}

	/// Returns a mutable reference to the memory mapping containing the given
	/// virtual address `ptr`. If no mapping contains the address, the function
	/// returns None.
	pub fn get_mapping_mut_for(&mut self, ptr: *const c_void) -> Option<&mut MemMapping> {
		Self::get_mapping_mut_for_(&mut self.mappings, ptr)
	}

	// TODO Optimize (currently O(n log n))
	/// Unmaps the given mapping of memory.
	/// `ptr` represents the aligned address of the beginning of the chunk to
	/// unmap. `size` represents the size of the mapping in number of memory
	/// pages. `brk` tells whether the function is called through the `brk`
	/// syscall. The function frees the physical memory the mapping points to
	/// unless shared by one or several other memory mappings.
	/// After this function returns, the access to the mapping of memory shall
	/// be revoked and further attempts to access it shall result in a page
	/// fault.
	pub fn unmap(&mut self, ptr: *const c_void, size: usize, brk: bool) -> Result<(), Errno> {
		if !util::is_aligned(ptr, memory::PAGE_SIZE) {
			return Err(errno!(EINVAL));
		}
		if size == 0 {
			return Ok(());
		}

		// Removing every mappings in the chunk to unmap
		let mut i = 0;
		while i < size {
			// The pointer of the page
			let page_ptr = (ptr as usize + i * memory::PAGE_SIZE) as *const _;

			// The mapping containing the page
			if let Some(mapping) = Self::get_mapping_mut_for_(&mut self.mappings, page_ptr) {
				// The pointer to the beginning of the mapping
				let mapping_ptr = mapping.get_begin();

				// The offset in the mapping of the beginning of pages to unmap
				let begin = (page_ptr as usize - mapping_ptr as usize) / memory::PAGE_SIZE;
				// The number of pages to unmap in the mapping
				let pages = min(size - i, mapping.get_size() - begin);

				// Removing the mapping
				let mapping = self.mappings.remove(mapping_ptr).unwrap();

				// Newly created mappings and gap after removing parts of the previous one
				let (prev, gap, next) = mapping.partial_unmap(begin, pages);

				if let Some(p) = prev {
					// TODO Merge with previous?
					oom::wrap(|| {
						let map = p.clone();
						self.mappings.insert(map.get_begin(), map)?;

						Ok(())
					});
				}

				if !brk {
					// Inserting gap
					if let Some(mut gap) = gap {
						self.vmem_usage -= gap.get_size();

						// Merging previous gap
						if !gap.get_begin().is_null() {
							let prev_gap =
								Self::gap_by_ptr(&self.gaps, unsafe { gap.get_begin().sub(1) });

							if let Some(p) = prev_gap {
								let begin = p.get_begin();
								let p = self.gap_remove(begin).unwrap();

								gap.merge(p);
							}
						}

						// Merging next gap
						let next_gap = Self::gap_by_ptr(&self.gaps, gap.get_end());
						if let Some(n) = next_gap {
							let begin = n.get_begin();
							let n = self.gap_remove(begin).unwrap();

							gap.merge(n);
						}

						oom::wrap(|| self.gap_insert(gap.clone()));
					}
				}

				if let Some(n) = next {
					// TODO Merge with next?
					oom::wrap(|| {
						let map = n.clone();
						self.mappings.insert(map.get_begin(), map)?;

						Ok(())
					});
				}

				i += pages;
			} else {
				i += 1;
			}
		}

		Ok(())
	}

	// TODO Optimize (use MMU)
	/// Tells whether the given mapping of memory `ptr` of size `size` in bytes
	/// can be accessed. `user` tells whether the memory must be accessible from
	/// userspace or just kernelspace. `write` tells whether to check for write
	/// permission.
	pub fn can_access(&self, ptr: *const u8, size: usize, user: bool, write: bool) -> bool {
		// TODO Allow reading kernelspace data that is available to userspace

		let mut i = 0;

		while i < size {
			// The beginning of the current page
			let page_begin = util::down_align((ptr as usize + i) as _, memory::PAGE_SIZE);

			if let Some(mapping) = Self::get_mapping_for_(&self.mappings, page_begin) {
				let flags = mapping.get_flags();
				if write && (flags & MAPPING_FLAG_WRITE == 0) {
					return false;
				}
				if user && (flags & MAPPING_FLAG_USER == 0) {
					return false;
				}

				i += mapping.get_size() * memory::PAGE_SIZE;
			} else {
				return false;
			}
		}

		true
	}

	// TODO Optimize (use MMU)
	/// Tells whether the given zero-terminated string beginning at `ptr` can be
	/// accessed. `user` tells whether the memory must be accessible from
	/// userspace or just kernelspace. `write` tells whether to check for write
	/// permission. If the memory cannot be accessed, the function returns None.
	/// If it can be accessed, it returns the length of the string located at
	/// the pointer `ptr`.
	pub fn can_access_string(&self, ptr: *const u8, user: bool, write: bool) -> Option<usize> {
		// TODO Allow reading kernelspace data that is available to userspace

		unsafe {
			vmem::switch(self.vmem.as_ref(), move || {
				let mut i = 0;
				'outer: loop {
					// Safe because not dereferenced before checking if accessible
					let curr_ptr = ptr.add(i);

					if let Some(mapping) = Self::get_mapping_for_(&self.mappings, curr_ptr as _) {
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

						// Looking for the null byte
						for j in 0..check_size {
							let c = *curr_ptr.add(j);

							// TODO Optimize by checking several bytes at a time
							if c == b'\0' {
								break 'outer;
							}

							i += 1;
						}
					} else {
						return None;
					}
				}

				Some(i)
			})
		}
	}

	/// Binds the CPU to this memory space.
	pub fn bind(&self) {
		self.vmem.bind();
	}

	/// Tells whether the memory space is bound.
	pub fn is_bound(&self) -> bool {
		self.vmem.is_bound()
	}

	/// Performs the fork operation.
	fn do_fork(&mut self) -> Result<Self, Errno> {
		let mut mem_space = Self {
			gaps: self.gaps.failable_clone()?,
			gaps_size: self.gaps_size_clone()?,

			mappings: Map::new(),

			vmem_usage: self.vmem_usage,

			brk_init: self.brk_init,
			brk_ptr: self.brk_ptr,

			vmem: vmem::clone(&self.vmem)?,
		};
		for (_, m) in self.mappings.iter_mut() {
			let new_mapping = m.fork(&mut mem_space)?;

			for i in 0..new_mapping.get_size() {
				m.update_vmem(i);
				new_mapping.update_vmem(i);
			}
		}

		Ok(mem_space)
	}

	/// Clones the current memory space for process forking.
	pub fn fork(&mut self) -> Result<MemSpace, Errno> {
		let mut result = Err(errno!(EINVAL));

		idt::wrap_disable_interrupts(|| unsafe {
			stack::switch(None, || {
				result = self.do_fork();
			})
		})?;

		result
	}

	/// Allocates the physical pages to write on the given pointer.
	/// `virt_addr` is the address to allocate.
	/// The size of the memory chunk to allocated equals `size_of::<T>() * len`.
	/// If the mapping doesn't exist, the function returns an error.
	pub fn alloc<T>(&mut self, virt_addr: *const T, len: usize) -> Result<(), Errno> {
		let mut off = 0;

		while off < size_of::<T>() * len {
			let virt_addr = (virt_addr as usize + off) as *const c_void;

			if let Some(mapping) = Self::get_mapping_mut_for_(&mut self.mappings, virt_addr) {
				let page_offset =
					(virt_addr as usize - mapping.get_begin() as usize) / memory::PAGE_SIZE;
				oom::wrap(|| mapping.map(page_offset));

				mapping.update_vmem(page_offset);
			} else {
				return Err(errno!(EINVAL));
			}

			off += util::up_align(virt_addr, memory::PAGE_SIZE) as usize - virt_addr as usize;
		}

		Ok(())
	}

	/// Returns the pointer for the `brk` syscall.
	pub fn get_brk_ptr(&self) -> *const c_void {
		self.brk_ptr
	}

	/// Sets the initial pointer for the `brk` syscall.
	/// This function MUST be called only once, before the program starts.
	/// `ptr` MUST be page-aligned.
	pub fn set_brk_init(&mut self, ptr: *const c_void) {
		debug_assert!(util::is_aligned(ptr, memory::PAGE_SIZE));

		self.brk_init = ptr;
		self.brk_ptr = ptr;
	}

	/// Sets the pointer for the `brk` syscall. If `alloc` is true, this
	/// function will allocate or free virtual memory if needed. If the memory
	/// cannot be allocated, the function returns an error.
	pub fn set_brk_ptr(&mut self, ptr: *const c_void) -> Result<(), Errno> {
		if ptr >= self.brk_ptr {
			// Allocate memory

			// Checking the pointer is valid
			if ptr > memory::PROCESS_END {
				return Err(errno!(ENOMEM));
			}

			let begin = util::align(self.brk_ptr, memory::PAGE_SIZE);
			let pages = math::ceil_division(ptr as usize - begin as usize, memory::PAGE_SIZE);
			let flags = MAPPING_FLAG_WRITE | MAPPING_FLAG_USER;

			self.map(MapConstraint::Fixed(begin), pages, flags, None)?;
		} else {
			// Free memory

			// Checking the pointer is valid
			if ptr < self.brk_init {
				return Err(errno!(ENOMEM));
			}

			let begin = util::align(ptr, memory::PAGE_SIZE);
			let pages = math::ceil_division(begin as usize - ptr as usize, memory::PAGE_SIZE);

			self.unmap(begin, pages, true)?;
		}

		self.brk_ptr = ptr;
		Ok(())
	}

	/// Function called whenever the CPU triggered a page fault for the context.
	/// This function determines whether the process should continue or not. If
	/// continuing, the function must resolve the issue before returning.
	/// A typical situation where is function is usefull is for Copy-On-Write
	/// allocations.
	///
	/// `virt_addr` is the virtual address of the wrong memory access that
	/// caused the fault. `code` is the error code given along with the error.
	/// If the process should continue, the function returns `true`, else
	/// `false`.
	pub fn handle_page_fault(&mut self, virt_addr: *const c_void, code: u32) -> bool {
		if code & vmem::x86::PAGE_FAULT_PRESENT == 0 {
			return false;
		}

		if let Some(mapping) = Self::get_mapping_mut_for_(&mut self.mappings, virt_addr) {
			let page_offset =
				(virt_addr as usize - mapping.get_begin() as usize) / memory::PAGE_SIZE;
			oom::wrap(|| mapping.map(page_offset));

			mapping.update_vmem(page_offset);
			true
		} else {
			false
		}
	}
}

impl fmt::Debug for MemSpace {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "Mappings:\n")?;
		for (_, m) in self.mappings.iter() {
			write!(f, "- {:?}\n", m)?;
		}

		write!(f, "\nGaps:\n")?;
		for (_, g) in self.gaps.iter() {
			write!(f, "- {:?}\n", g)?;
		}

		Ok(())
	}
}

impl Drop for MemSpace {
	fn drop(&mut self) {
		if self.is_bound() {
			kernel_panic!("Dropping a memory space while bound to it");
		}

		// Unmapping everything to free up physical memory
		for (_, m) in self.mappings.iter_mut() {
			oom::wrap(|| m.unmap());
		}
	}
}
