/// This module implements the MemSpace structure which is responsible for handling the memory
/// mapping of execution contexts.
/// TODO doc

mod gap;
mod mapping;

use core::cmp::min;
use core::ffi::c_void;
use crate::memory::vmem::VMem;
use crate::memory::vmem;
use crate::memory;
use crate::util::boxed::Box;
use crate::util::container::binary_tree::BinaryTree;
use crate::util::list::List;
use crate::util::math;
use gap::MemGap;
use mapping::MemMapping;

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

/// The number of buckets for available gaps in memory.
const GAPS_BUCKETS_COUNT: usize = 8;

/// Structure representing the virtual memory space of a context.
pub struct MemSpace {
	/// Binary tree storing the list of memory gaps, ready for new mappings. Sorted by pointer to
	/// the beginning of the mapping on the virtual memory.
	gaps: BinaryTree::<MemGap>,
	/// The gaps bucket, sorted by size. The minimum size in pages of a gap is: `2^^n`, where `n`
	/// is the index in the list.
	gaps_buckets: [List::<MemGap>; GAPS_BUCKETS_COUNT],

	/// Binary tree storing the list of memory mappings. Sorted by pointer to the beginning of the
	/// mapping on the virtual memory.
	mappings: BinaryTree::<MemMapping>,

	/// The virtual memory context handler.
	vmem: Box::<dyn VMem>,
}

impl MemSpace {
	/// Returns the bucket index for a gap of size `size`.
	fn get_gap_bucket_index(size: usize) -> usize {
		min(math::log2(size), GAPS_BUCKETS_COUNT - 1)
	}

	/// Inserts the given gap into the memory space's structures.
	fn gap_insert(&mut self, gap: MemGap) -> Result::<(), ()> {
		let gap_ptr = gap.get_begin();
		self.gaps.insert(gap)?;
		let g = self.gaps.get(gap_ptr).unwrap();

		let bucket_index = Self::get_gap_bucket_index(g.get_size());
		let bucket = &mut self.gaps_buckets[bucket_index];
		bucket.insert_front(&mut g.list);

		Ok(())
	}

	/// Removes the given gap from the memory space's structures.
	fn gap_remove(&mut self, gap_begin: *const c_void) {
		let g = self.gaps.get(gap_begin).unwrap();

		let bucket_index = Self::get_gap_bucket_index(g.get_size());
		let bucket = &mut self.gaps_buckets[bucket_index];
		g.list.unlink_from(bucket);

		self.gaps.remove(gap_begin);
	}

	/// Returns a reference to a gap with at least size `size`.
	fn gap_get(buckets: &mut [List::<MemGap>], size: usize) -> Option::<&mut MemGap> {
		let bucket_index = Self::get_gap_bucket_index(size);

		for i in bucket_index..GAPS_BUCKETS_COUNT {
			let bucket = &mut buckets[i];

			let mut node = bucket.get_front();
			while node.is_some() {
				let n = node.unwrap();
				let value = n.get_mut::<MemGap>(bucket.get_inner_offset());
				if value.get_size() >= size {
					return Some(value);
				}
				node = n.get_next();
			}
		}

		None
	}

	/// Returns a new binary tree containing the default gaps for a memory space.
	fn create_default_gaps(&mut self) -> Result::<(), ()> {
		let begin = memory::ALLOC_BEGIN;
		let size = (memory::PROCESS_END as usize - begin as usize) / memory::PAGE_SIZE;
		self.gap_insert(MemGap::new(begin, size))
	}

	/// Creates a new virtual memory object.
	pub fn new() -> Result::<Self, ()> {
		let mut s = Self {
			gaps: BinaryTree::new(),
			gaps_buckets: [crate::list_new!(MemGap, list); GAPS_BUCKETS_COUNT],

			mappings: BinaryTree::new(),

			vmem: vmem::new()?,
		};
		s.create_default_gaps()?;
		Ok(s)
	}

	/// Maps a region of memory.
	/// `ptr` represents the address of the beginning of the region on the virtual memory.
	/// If the address is None, the function shall find a gap in the memory space that is large
	/// enough to contain the mapping.
	/// `size` represents the size of the region in number of memory pages.
	/// `flags` represents the flags for the mapping.
	/// underlying physical memory is not allocated directly but only an attempt to write the
	/// memory is detected.
	/// The function returns a pointer to the newly mapped virtual memory.
	pub fn map(&mut self, ptr: Option::<*const c_void>, size: usize, flags: u8)
		-> Result::<*const c_void, ()> {
		if let Some(_ptr) = ptr {
			// TODO Insert mapping at exact location if possible
			Err(())
		} else {
			let gap = Self::gap_get(&mut self.gaps_buckets, size);
			if gap.is_none() {
				return Err(());
			}

			let gap = gap.unwrap();
			let gap_ptr = gap.get_begin();

			let mapping = MemMapping::new(gap_ptr, size, flags);
			let mapping_ptr = mapping.get_begin();
			self.mappings.insert(mapping)?;

			if self.mappings.get(mapping_ptr).unwrap().map_default(&mut self.vmem).is_err() {
				self.mappings.remove(mapping_ptr);
				return Err(());
			}

			if let Some(new_gap) = gap.consume(size) {
				if self.gap_insert(new_gap).is_err() {
					self.mappings.get(mapping_ptr).unwrap().unmap(&mut self.vmem);
					self.mappings.remove(mapping_ptr);
					return Err(());
				}
			}

			self.gap_remove(gap_ptr);
			Ok(mapping_ptr)
		}
	}

	/// Same as `map`, except the function returns a pointer to the end of the memory region.
	pub fn map_stack(&mut self, ptr: Option::<*const c_void>, size: usize, flags: u8)
		-> Result::<*const c_void, ()> {
		let mapping_ptr = self.map(ptr, size, flags)?;
		Ok(unsafe { // Call to unsafe function
			mapping_ptr.add(size * memory::PAGE_SIZE) // `- 1`?
		})
	}

	/// Unmaps the given region of memory.
	/// `ptr` represents the address of the beginning of the region on the virtual memory.
	/// `size` represents the size of the region in number of memory pages.
	/// The function frees the physical memory the region points to unless shared by one or several
	/// other memory mappings.
	/// After this function returns, the access to the region of memory shall be revoked and
	/// further attempts to access it shall result in a page fault.
	pub fn unmap(&mut self, _ptr: *const c_void, _size: usize) {
		// TODO
	}

	/// Binds the CPU to this memory space.
	pub fn bind(&self) {
		self.vmem.bind();
	}

	/// Function called whenever the CPU triggered a page fault for the context. This function
	/// determines whether the process should continue or not. If continuing, the function must
	/// resolve the issue before returning.
	/// A typical situation where is function is usefull is for Copy-On-Write allocations.
	///
	/// `virt_addr` is the virtual address of the wrong memory access that caused the fault.
	/// If the process should continue, the function returns `true`, else `false`.
	pub fn handle_page_fault(&self, _virt_addr: *const c_void) -> bool {
		// TODO
		false
	}
}

impl Drop for MemSpace {
	fn drop(&mut self) {
		// TODO Free every allocations
	}
}
