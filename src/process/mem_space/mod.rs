//! This module implements the MemSpace structure which is responsible for handling the memory
//! mapping of execution contexts.
//!
//! The memory space contains two types of structures:
//! - Mapping: A chunk of virtual memory that is allocated
//! - Gap: A chunk of virtual memory that is available to be allocated

mod gap;
mod mapping;
mod physical_ref_counter;

use core::cmp::Ordering;
use core::cmp::{min, max};
use core::ffi::c_void;
use core::mem::replace;
use core::mem::size_of;
use core::ptr::NonNull;
use crate::errno::Errno;
use crate::errno;
use crate::file::file_descriptor::FileDescriptor;
use crate::memory::stack;
use crate::memory::vmem::VMem;
use crate::memory::vmem;
use crate::memory;
use crate::process::oom;
use crate::util::FailableClone;
use crate::util::boxed::Box;
use crate::util::container::binary_tree::BinaryTree;
use crate::util::lock::Mutex;
use crate::util;
use gap::MemGap;
use mapping::MemMapping;
use physical_ref_counter::PhysRefCounter;

/// Flag telling that a memory mapping can be written to.
pub const MAPPING_FLAG_WRITE: u8  = 0b00001;
/// Flag telling that a memory mapping can contain executable instructions.
pub const MAPPING_FLAG_EXEC: u8   = 0b00010;
/// Flag telling that a memory mapping is accessible from userspace.
pub const MAPPING_FLAG_USER: u8   = 0b00100;
/// Flag telling that a memory mapping must allocate its physical memory right away and not when
/// the process tries to write to it.
pub const MAPPING_FLAG_NOLAZY: u8 = 0b01000;
/// Flag telling that a memory mapping has its physical memory shared with one or more other
/// mappings.
pub const MAPPING_FLAG_SHARED: u8 = 0b10000;

/// The size of the temporary stack used to fork a memory space.
const TMP_STACK_SIZE: usize = memory::PAGE_SIZE * 8;

/// The physical pages reference counter.
pub static mut PHYSICAL_REF_COUNTER: Mutex<PhysRefCounter> = Mutex::new(PhysRefCounter::new());

/// Structure representing the data passed to the temporary stack used to fork a memory space.
/// It is necessary to switch stacks because using a stack while mapping it is undefined.
struct ForkData<'a> {
	/// A reference to the memory space.
	self_: &'a mut MemSpace,

	/// The result of the mapping operation.
	result: Result<MemSpace, Errno>,
}

/// Structure representing the virtual memory space of a context.
pub struct MemSpace {
	/// Binary tree storing the list of memory gaps, ready for new mappings. Sorted by pointer to
	/// the beginning of the mapping on the virtual memory.
	gaps: BinaryTree<*const c_void, MemGap>,
	/// Binary tree storing the list of memory gaps, sorted by size. The key is the size of the gap
	/// and the value is the pointer to its beginning.
	gaps_size: BinaryTree<usize, *const c_void>,

	/// Binary tree storing the list of memory mappings. Sorted by pointer to the beginning of the
	/// mapping on the virtual memory.
	mappings: BinaryTree<*const c_void, MemMapping>,

	/// The virtual memory context handler.
	vmem: Box<dyn VMem>,
}

impl MemSpace {
	/// Inserts the given gap into the memory space's structures.
	fn gap_insert(&mut self, gap: MemGap) -> Result<(), Errno> {
		let gap_ptr = gap.get_begin();
		let g = self.gaps.insert(gap_ptr, gap)?;
		self.gaps_size.insert(g.get_size(), gap_ptr)?;

		Ok(())
	}

	/// Removes the given gap from the memory space's structures.
	fn gap_remove(&mut self, gap_begin: *const c_void) {
		let g = self.gaps.remove(gap_begin).unwrap();
		self.gaps_size.select_remove(g.get_size(), | val | {
			*val == gap_begin
		});
	}

	/// Returns a reference to a gap with at least size `size`.
	/// `gaps` is the binary tree storing gaps, sorted by pointer to their respective beginnings.
	/// `gaps_size` is the binary tree storing pointers to gaps, sorted by gap sizes.
	/// `size` is the minimum size of the gap.
	/// If no gap large enough is available, the function returns None.
	fn gap_get<'a>(gaps: &'a BinaryTree<*const c_void, MemGap>, gaps_size: &BinaryTree<usize,
		*const c_void>, size: usize) -> Option<&'a MemGap> {
		let ptr = gaps_size.get_min(size)?.1;
		let gap = gaps.get(*ptr).unwrap();
		debug_assert!(gap.get_size() >= size);

		Some(gap)
	}

	/// Returns a reference to the gap containing the pointer `ptr`.
	/// `gaps` is the binary tree storing gaps, sorted by pointer to their respective beginnings.
	/// `ptr` is the pointer.
	/// If no gap contain the pointer, the function returns None.
	fn gap_by_ptr<'a>(gaps: &'a BinaryTree<*const c_void, MemGap>, ptr: *const c_void)
		-> Option<&'a MemGap> {
		gaps.cmp_get(| key, value | {
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

	/// Returns a new binary tree containing the default gaps for a memory space.
	fn create_default_gaps(&mut self) -> Result<(), Errno> {
		let begin = memory::ALLOC_BEGIN;
		let size = (memory::PROCESS_END as usize - begin as usize) / memory::PAGE_SIZE;
		self.gap_insert(MemGap::new(begin, size))
	}

	/// Creates a new virtual memory object.
	pub fn new() -> Result::<Self, Errno> {
		let mut s = Self {
			gaps: BinaryTree::new(),
			gaps_size: BinaryTree::new(),

			mappings: BinaryTree::new(),

			vmem: vmem::new()?,
		};
		s.create_default_gaps()?;
		Ok(s)
	}

	/// Returns a mutable reference to the vvirtual memory context.
	pub fn get_vmem(&mut self) -> &mut Box<dyn VMem> {
		&mut self.vmem
	}

	/// Maps a chunk of memory.
	/// `ptr` represents the address of the beginning of the mapping on the virtual memory.
	/// If the address is None, the function shall find a gap in the memory space that is large
	/// enough to contain the mapping.
	/// `size` represents the size of the mapping in number of memory pages.
	/// `flags` represents the flags for the mapping.
	/// `fd` is the file descriptor pointing to the file to map to.
	/// `fd_off` is the offset in bytes into the file.
	/// The underlying physical memory is not allocated directly but only an attempt to write the
	/// memory is detected.
	/// The function returns a pointer to the newly mapped virtual memory.
	/// The function has complexity `O(log n)`.
	pub fn map(&mut self, ptr: Option<*const c_void>, size: usize, flags: u8,
		fd: Option<FileDescriptor>, fd_off: usize) -> Result<*const c_void, Errno> {
		if size <= 0 {
			return Err(errno::EINVAL);
		}

		// The gap to use and the offset in said gap
		let (gap, off) = {
			if let Some(ptr) = ptr {
				// Unmapping memory previously mapped at this location
				self.unmap(ptr, size)?; // FIXME Must be undone on fail

				// Getting the gap for the pointer
				let gap = Self::gap_by_ptr(&self.gaps, ptr).ok_or(errno::ENOMEM)?;

				// The offset in the gap
				let off = (gap.get_begin() as usize - ptr as usize) / memory::PAGE_SIZE;
				if size > gap.get_size() - off {
					return Err(errno::ENOMEM);
				}

				(gap, off)
			} else {
				// Getting a gap large enough
				let gap = Self::gap_get(&self.gaps, &self.gaps_size, size).ok_or(errno::ENOMEM)?;

				(gap, 0)
			}
		};

		// The address to the beginning of the mapping
		let addr = (gap.get_begin() as usize + off * memory::PAGE_SIZE) as _;

		// FIXME Adjust size to non-aligned memory addresses
		// Creating the mapping
		let mapping = MemMapping::new(addr, size, flags, fd, fd_off,
			NonNull::new(self.vmem.as_mut_ptr()).unwrap());
		let mapping_ptr = mapping.get_begin();
		debug_assert!(ptr.is_none() || mapping_ptr == ptr.unwrap());

		let m = self.mappings.insert(mapping_ptr, mapping)?;

		// Mapping the default page
		if m.map_default().is_err() {
			self.mappings.remove(mapping_ptr);
			return Err(errno::ENOMEM);
		}

		// Splitting the old gap to fit the mapping
		let (left_gap, right_gap) = gap.consume(off, size);

		// Removing the old gap
		let gap_begin = gap.get_begin();
		self.gap_remove(gap_begin);

		// Inserting the new gaps
		oom::wrap(|| {
			if let Some(new_gap) = &left_gap {
				self.gap_insert(new_gap.clone())?;
			}
			if let Some(new_gap) = &right_gap {
				self.gap_insert(new_gap.clone())?;
			}

			Ok(())
		});

		Ok(mapping_ptr)
	}

	/// Same as `map`, except the function returns a pointer to the end of the memory mapping.
	pub fn map_stack(&mut self, ptr: Option<*const c_void>, size: usize, flags: u8)
		-> Result<*const c_void, Errno> {
		let mapping_ptr = self.map(ptr, size, flags, None, 0)?;
		Ok(unsafe { // Safe because the new pointer stays in the range of the allocated mapping
			mapping_ptr.add(size * memory::PAGE_SIZE)
		})
	}

	/// Returns a reference to the memory mapping containing the given virtual address `ptr` from
	/// mappings container `mappings`. If no mapping contains the address, the function returns
	/// None.
	fn get_mapping_for_(mappings: &BinaryTree<*const c_void, MemMapping>, ptr: *const c_void)
		-> Option<&MemMapping> {
		mappings.cmp_get(| key, value | {
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

	/// Returns a mutable reference to the memory mapping containing the given virtual address
	/// `ptr` from mappings container `mappings`. If no mapping contains the address, the function
	/// returns None.
	fn get_mapping_mut_for_(mappings: &mut BinaryTree<*const c_void, MemMapping>,
		ptr: *const c_void) -> Option<&mut MemMapping> {
		mappings.cmp_get_mut(| key, value | {
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

	/// Returns a mutable reference to the memory mapping containing the given virtual address
	/// `ptr`. If no mapping contains the address, the function returns None.
	pub fn get_mapping_mut_for(&mut self, ptr: *const c_void) -> Option<&mut MemMapping> {
		Self::get_mapping_mut_for_(&mut self.mappings, ptr)
	}

	/// Creates the gap to unmap a chunk of memory.
	/// `ptr` represents the address of the beginning of the chunk to unmap.
	/// `size` represents the size of the mapping in number of memory pages.
	fn create_unmap_gap(&self, ptr: *const c_void, size: usize) -> MemGap {
		// The pointer to the beginning of the chunk to be unmapped
		let mut gap_begin = util::down_align(ptr, memory::PAGE_SIZE);
		// The gap's size in bytes
		let gap_size = size * memory::PAGE_SIZE;
		// The size of the new gap
		let mut gap_end = unsafe {
			gap_begin.add(size * memory::PAGE_SIZE)
		};

		// The previous gap, located before the gap being created
		let prev_gap = Self::gap_by_ptr(&self.gaps, unsafe {
			gap_begin.sub(1)
		});
		// The next gap, located after the gap being created
		let next_gap = Self::gap_by_ptr(&self.gaps, unsafe {
			gap_begin.add(gap_size * memory::PAGE_SIZE)
		});

		// Expanding to absorb the previous gap
		if let Some(prev_gap) = prev_gap {
			// The gap's size in bytes
			let size = prev_gap.get_size() * memory::PAGE_SIZE;

			gap_begin = min(gap_begin, prev_gap.get_begin());
			gap_end = max(gap_end, unsafe {
				prev_gap.get_begin().add(size)
			});
		}

		// Expanding to absorb the next gap
		if let Some(next_gap) = next_gap {
			// The gap's size in bytes
			let size = next_gap.get_size() * memory::PAGE_SIZE;

			gap_begin = min(gap_begin, next_gap.get_begin());
			gap_end = max(gap_end, unsafe {
				next_gap.get_begin().add(size)
			});
		}

		MemGap::new(gap_begin, (gap_end as usize - gap_begin as usize) / memory::PAGE_SIZE)
	}

	// TODO Optimize (currently O(n log n), can be reduced to O(log n) by avoiding to make a new
	// binary tree search at each iterations)
	/// Unmaps the given mapping of memory.
	/// `ptr` represents the address of the beginning of the chunk to unmap.
	/// `size` represents the size of the mapping in number of memory pages.
	/// The function frees the physical memory the mapping points to unless shared by one or
	/// several other memory mappings.
	/// After this function returns, the access to the mapping of memory shall be revoked and
	/// further attempts to access it shall result in a page fault.
	/// If `ptr` is not aligned, the behaviour is undefined.
	pub fn unmap(&mut self, ptr: *const c_void, size: usize) -> Result<(), Errno> {
		debug_assert!(util::is_aligned(ptr, memory::PAGE_SIZE));

		if size <= 0 {
			return Ok(());
		}

		// Removing every regions in the chunk to unmap
		let mut i = 0;
		while i < size {
			// The pointer of the page
			let page_ptr = (ptr as usize + i * memory::PAGE_SIZE) as *const _;

			// The mapping containing the page
			if let Some(mapping) = Self::get_mapping_mut_for_(&mut self.mappings, page_ptr) {
				// The offset in the mapping of the beginning of pages to unmap
				let begin = (page_ptr as usize - mapping.get_begin() as usize)
					/ memory::PAGE_SIZE;
				// The number of pages to unmap in the mapping
				let pages = min(size - i, mapping.get_size() - begin);

				// The pointer to the beginning of the mapping
				let mapping_begin_ptr = mapping.get_begin();
				// The mapping
				let mapping = self.mappings.remove(mapping_begin_ptr).unwrap();

				// Newly created mappings after removing parts of the previous one
				let (prev, next) = mapping.partial_unmap(begin, pages);
				if let Some(p) = prev {
					oom::wrap(|| {
						self.mappings.insert(p.get_begin(), p.clone())?;
						Ok(())
					});
				}
				if let Some(n) = next {
					oom::wrap(|| {
						self.mappings.insert(n.get_begin(), n.clone())?;
						Ok(())
					});
				}

				i += pages;
			} else {
				i += 1;
			}
		}

		// Removing gaps already present in the chunk to unmap
		let mut i = 0;
		while i < size {
			// The current pointer
			let begin = unsafe {
				ptr.add(i * memory::PAGE_SIZE)
			};

			// If a gap is located at the pointer `begin`, remove it
			if let Some(g) = self.gaps.remove(begin) {
				let size = g.get_size();
				self.gaps_size.select_remove(size, | val | {
					*val == begin
				});

				i += size;
			} else {
				i += 1;
			}
		}

		// The new gap covering the unmapped chunk
		oom::wrap(|| {
			let gap = self.create_unmap_gap(ptr, size);
			self.gap_insert(gap)
		});

		// Unmapping the chunk from virtual memory
		let vmem = self.get_vmem();
		oom::wrap(|| {
			vmem.unmap_range(ptr, size)
		});

		Ok(())
	}

	/// Tells whether the given mapping of memory `ptr` of size `size` in bytes can be accessed.
	/// `user` tells whether the memory must be accessible from userspace or just kernelspace.
	/// `write` tells whether to check for write permission.
	pub fn can_access(&self, ptr: *const u8, size: usize, user: bool, write: bool) -> bool {
		// TODO Allow reading kernelspace data that is available to userspace

		let mut i = 0;

		while i < size {
			// The beginning of the current page
			let page_begin = util::down_align((ptr as usize + i) as _, memory::PAGE_SIZE);

			if let Some(mapping) = Self::get_mapping_for_(&self.mappings, page_begin) {
				let flags = mapping.get_flags();
				if write && !(flags & MAPPING_FLAG_WRITE != 0) {
					return false;
				}
				if user && !(flags & MAPPING_FLAG_USER != 0) {
					return false;
				}

				i += mapping.get_size() * memory::PAGE_SIZE;
			} else {
				return false;
			}
		}

		true
	}

	/// Tells whether the given zero-terminated string beginning at `ptr` can be accessed.
	/// `user` tells whether the memory must be accessible from userspace or just kernelspace.
	/// `write` tells whether to check for write permission.
	/// If the memory cannot be accessed, the function returns None. If it can be accessed, it
	/// returns the length of the string located at the pointer `ptr`.
	pub fn can_access_string(&self, ptr: *const u8, user: bool, write: bool) -> Option<usize> {
		// TODO Allow reading kernelspace data that is available to userspace

		vmem::switch(self.vmem.as_ref(), || {
			let mut i = 0;
			'outer: loop {
				// Safe because not dereferenced before checking if accessible
				let curr_ptr = unsafe {
					ptr.add(i)
				};

				if let Some(mapping) = Self::get_mapping_for_(&self.mappings, curr_ptr as _) {
					let flags = mapping.get_flags();
					if write && !(flags & MAPPING_FLAG_WRITE != 0) {
						return None;
					}
					if user && !(flags & MAPPING_FLAG_USER != 0) {
						return None;
					}

					// The beginning of the current page
					let page_begin = util::down_align(curr_ptr as _, memory::PAGE_SIZE);
					// The offset of the current pointer in its page
					let inner_off = curr_ptr as usize - page_begin as usize;
					let check_size = memory::PAGE_SIZE - inner_off;

					for j in 0..check_size {
						let c = unsafe { // Safe because the pointer is checked before
							*curr_ptr.add(j)
						};

						// TODO Optimize by checking several bytes at a time
						if c == b'\0' {
							break 'outer;
						}
					}

					i += check_size;
				} else {
					return None;
				}
			}

			Some(i)
		})
	}

	/// Binds the CPU to this memory space.
	pub fn bind(&self) {
		self.vmem.bind();
	}

	/// Tells whether the memory space is bound.
	pub fn is_bound(&self) -> bool {
		self.vmem.is_bound()
	}

	/// Performs the actions of `fork`. This function is meant to be called onto a temporary stack.
	fn do_fork(&mut self) -> Result<MemSpace, Errno> {
		let mut mem_space = Self {
			gaps: self.gaps.failable_clone()?,
			gaps_size: self.gaps_size.failable_clone()?,

			mappings: BinaryTree::new(),

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
		let tmp_stack = Box::<[u8; TMP_STACK_SIZE]>::new([0; TMP_STACK_SIZE])?;
		let tmp_stack_top = unsafe {
			(tmp_stack.as_ptr() as *mut c_void).add(TMP_STACK_SIZE)
		};

		let f: fn(*mut c_void) -> () = | data: *mut c_void | {
			let data = unsafe {
				&mut *(data as *mut ForkData)
			};
			data.result = data.self_.do_fork();
		};

		let mut fork_data = Box::new(ForkData {
			self_: self,

			result: Err(0),
		})?;
		unsafe {
			stack::switch(tmp_stack_top, f, fork_data.as_mut_ptr());
		}
		replace(&mut fork_data.result, Err(0))
	}

	/// Allocates the physical pages to write on the given pointer.
	/// `virt_addr` is the address to allocate.
	/// `size` is the size of the mapping to allocate.
	/// If the mapping doesn't exist, the function returns an error.
	pub fn alloc<T>(&mut self, virt_addr: *const T) -> Result<(), Errno> {
		let mut off = 0;

		while off < size_of::<T>() {
			let virt_addr = (virt_addr as usize + off) as *const c_void;

			if let Some(mapping) = Self::get_mapping_mut_for_(&mut self.mappings, virt_addr) {
				let page_offset = (virt_addr as usize - mapping.get_begin() as usize)
					/ memory::PAGE_SIZE;
				oom::wrap(|| {
					mapping.map(page_offset)
				});

				mapping.update_vmem(page_offset);
			} else {
				return Err(errno::EINVAL);
			}

			off += util::up_align(virt_addr, memory::PAGE_SIZE) as usize - virt_addr as usize;
		}

		Ok(())
	}

	/// Function called whenever the CPU triggered a page fault for the context. This function
	/// determines whether the process should continue or not. If continuing, the function must
	/// resolve the issue before returning.
	/// A typical situation where is function is usefull is for Copy-On-Write allocations.
	///
	/// `virt_addr` is the virtual address of the wrong memory access that caused the fault.
	/// `code` is the error code given along with the error.
	/// If the process should continue, the function returns `true`, else `false`.
	pub fn handle_page_fault(&mut self, virt_addr: *const c_void, code: u32) -> bool {
		if code & vmem::x86::PAGE_FAULT_PRESENT == 0 {
			return false;
		}

		if let Some(mapping) = Self::get_mapping_mut_for_(&mut self.mappings, virt_addr) {
			let page_offset = (virt_addr as usize - mapping.get_begin() as usize)
				/ memory::PAGE_SIZE;
			oom::wrap(|| {
				mapping.map(page_offset)
			});

			mapping.update_vmem(page_offset);
			true
		} else {
			false
		}
	}
}
