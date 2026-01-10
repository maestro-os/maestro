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
pub mod mapping;
mod transaction;

use crate::{
	arch::{
		core_id, x86,
		x86::paging::{PAGE_FAULT_INSTRUCTION, PAGE_FAULT_WRITE},
	},
	file::{File, vfs},
	memory::{
		COMPAT_PROCESS_END, PROCESS_END, VirtAddr,
		cache::RcFrame,
		user::UserSlice,
		vmem::{KERNEL_VMEM, VMem, shootdown_range},
	},
	process::{
		mem_space::mapping::MappedFrame,
		scheduler::{cpu, cpu::per_cpu, critical},
	},
	sync::rwlock::IntRwLock,
};
use core::{
	alloc::AllocError, cmp::min, ffi::c_void, fmt, hint::unlikely, mem, num::NonZeroUsize,
};
use gap::MemGap;
use mapping::MemMapping;
use transaction::MemSpaceTransaction;
use utils::{
	TryClone,
	collections::{btreemap::BTreeMap, vec::Vec},
	errno,
	errno::{AllocResult, CollectResult, EResult},
	limits::PAGE_SIZE,
	ptr::arc::Arc,
	range_cmp,
};

/// Page can be read
pub const PROT_READ: u8 = 0x1;
/// Page can be written
pub const PROT_WRITE: u8 = 0x2;
/// Page can be executed
pub const PROT_EXEC: u8 = 0x4;

/// Changes are shared across mappings on the same region
pub const MAP_SHARED: i32 = 0x1;
/// Changes are not carried to the underlying file
pub const MAP_PRIVATE: i32 = 0x2;
/// Interpret `addr` exactly
pub const MAP_FIXED: i32 = 0x10;
/// The mapping is not backed by any file
pub const MAP_ANONYMOUS: i32 = 0x20;
/// Interpret `addr` exactly, failing if already used
pub const MAP_FIXED_NOREPLACE: i32 = 0x100000;

/// The virtual address of the buffer used to map pages for copy.
const COPY_BUFFER: VirtAddr = VirtAddr(PROCESS_END.0 - PAGE_SIZE);

/// Type representing a memory page.
pub type Page = [u8; PAGE_SIZE];

/// Tells whether the address is in bound of the userspace.
pub fn bound_check(addr: usize, n: usize) -> bool {
	addr >= PAGE_SIZE && addr.saturating_add(n) <= COPY_BUFFER.0
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
		.state
		.get_gap_for_addr(start)
		.map(MemGap::get_begin)
		// No gap contain the start address, start at the next one
		.unwrap_or(start);
	// Bound the search to the end of the range
	let end = start + size * PAGE_SIZE;
	// Collect gaps that match
	let gaps = transaction
		.state
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
		let off = start.0.saturating_sub(gap_begin.0) / PAGE_SIZE;
		let end = end.0.clamp(gap_begin.0, gap_end.0) / PAGE_SIZE;
		// Consume the gap and store new gaps
		let (prev, next) = gap.consume(off, end - off);
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
#[derive(Default, Debug)]
struct MemSpaceState {
	/// Binary tree storing the list of memory gaps, ready for new mappings.
	///
	/// The collection is sorted by pointer to the beginning of the mapping on the virtual
	/// memory.
	gaps: BTreeMap<VirtAddr, MemGap>,
	/// Binary tree storing the list of memory mappings.
	///
	/// Sorted by pointer to the beginning of the mapping on the virtual memory.
	mappings: BTreeMap<VirtAddr, MemMapping>,

	/// The initial pointer of the `[s]brk` system calls.
	brk_init: VirtAddr,
	/// The current pointer of the `[s]brk` system calls.
	brk: VirtAddr,

	/// The number of used virtual memory pages.
	vmem_usage: usize,
}

impl MemSpaceState {
	/// Returns a reference to a gap with at least size `size`.
	///
	/// `size` is the minimum size of the gap to be returned.
	///
	/// If no gap large enough is available, the function returns `None`.
	fn get_gap(&self, size: NonZeroUsize) -> Option<&MemGap> {
		// TODO iterate starting from the end to minimize the likelihood to collide with brk
		self.gaps
			.iter()
			.map(|(_, g)| g)
			.find(|g| g.get_size() >= size)
	}

	/// Returns a reference to the gap containing the given virtual address.
	///
	/// If no gap contain the pointer, the function returns `None`.
	fn get_gap_for_addr(&self, addr: VirtAddr) -> Option<&MemGap> {
		self.gaps
			.cmp_get(|key, value| range_cmp(key.0, value.get_size().get() * PAGE_SIZE, addr.0))
	}

	/// Returns an immutable reference to the memory mapping containing the given virtual
	/// address.
	///
	/// If no mapping contains the address, the function returns `None`.
	pub fn get_mapping_for_addr(&self, addr: VirtAddr) -> Option<&MemMapping> {
		self.mappings
			.cmp_get(|key, value| range_cmp(key.0, value.size.get() * PAGE_SIZE, addr.0))
	}

	/// Returns a mutable reference to the memory mapping containing the given virtual
	/// address.
	///
	/// If no mapping contains the address, the function returns `None`.
	pub fn get_mut_mapping_for_addr(&mut self, addr: VirtAddr) -> Option<&mut MemMapping> {
		self.mappings
			.cmp_get_mut(|key, value| range_cmp(key.0, value.size.get() * PAGE_SIZE, addr.0))
	}
}

/// Executable program information.
#[derive(Clone)]
pub struct ExeInfo {
	/// The VFS entry of the program loaded on this memory space.
	pub exe: Arc<vfs::Entry>,

	/// Address to the beginning of program argument.
	pub argv_begin: VirtAddr,
	/// Address to the end of program argument.
	pub argv_end: VirtAddr,
	/// Address to the beginning of program environment.
	pub envp_begin: VirtAddr,
	/// Address to the end of program environment.
	pub envp_end: VirtAddr,
}

/// A virtual memory space.
pub struct MemSpace {
	/// The memory space's structure, used as a model for `vmem`
	state: IntRwLock<MemSpaceState>,
	/// Architecture-specific virtual memory context handler
	///
	/// We use it as a cache which can be invalidated by unmapping. When a page fault occurs, this
	/// field is corrected by the [`MemSpace`].
	vmem: VMem,

	/// Executable program information
	pub exe_info: ExeInfo,

	/// Bitmap of CPUs currently binding the memory space
	bound_cpus: cpu::Bitmap,
}

impl MemSpace {
	/// Creates a new virtual memory object.
	///
	/// Arguments:
	/// - `exe` is the VFS entry of the program loaded on the memory space
	/// - `brk_init` is the base address at which `brk` begins
	/// - `compat` tells whether the memory space be used in compat mode
	pub fn new(exe: Arc<vfs::Entry>, brk_init: VirtAddr, compat: bool) -> AllocResult<Arc<Self>> {
		let s = Self {
			state: IntRwLock::new(MemSpaceState {
				brk_init,
				brk: brk_init,
				..Default::default()
			}),
			vmem: unsafe { VMem::new() },

			exe_info: ExeInfo {
				exe,

				argv_begin: Default::default(),
				argv_end: Default::default(),
				envp_begin: Default::default(),
				envp_end: Default::default(),
			},

			bound_cpus: cpu::Bitmap::new(false)?,
		};
		// Allocation begin and end addresses
		let begin = VirtAddr(PAGE_SIZE);
		let end = if compat {
			COMPAT_PROCESS_END - PAGE_SIZE
		} else {
			COPY_BUFFER
		};
		// Create the default gap of memory which is present at the beginning
		let size = (end.0 - begin.0) / PAGE_SIZE;
		let gap = MemGap::new(begin, NonZeroUsize::new(size).unwrap());
		let mut transaction = MemSpaceTransaction::new(&s);
		transaction.insert_gap(gap)?;
		transaction.commit();
		Arc::new(s)
	}

	/// Returns the number of virtual memory pages in the memory space.
	#[inline]
	pub fn get_vmem_usage(&self) -> usize {
		self.state.read().vmem_usage
	}

	/// Locks the list of mappings while executing `f`, passing it as parameter.
	#[inline]
	pub fn mappings<T, F: FnOnce(&BTreeMap<VirtAddr, MemMapping>) -> T>(&self, f: F) -> T {
		let state = self.state.read();
		f(&state.mappings)
	}

	fn map_impl(
		transaction: &mut MemSpaceTransaction,
		addr: VirtAddr,
		size: NonZeroUsize,
		prot: u8,
		flags: i32,
		file: Option<Arc<File>>,
		off: u64,
	) -> EResult<MemMapping> {
		if unlikely(!addr.is_aligned_to(PAGE_SIZE)) {
			return Err(errno!(EINVAL));
		}
		if size
			.get()
			.checked_mul(PAGE_SIZE)
			.and_then(|l| addr.checked_add(l))
			.is_none_or(|end| end > COPY_BUFFER.0)
		{
			return Err(errno!(EINVAL));
		};
		if unlikely(flags & (MAP_PRIVATE | MAP_SHARED) == 0) {
			return Err(errno!(EINVAL));
		}
		if flags & MAP_FIXED_NOREPLACE != 0 {
			// Check for mappings already present in range TODO: can be optimized
			let used = transaction.state.mappings.iter().any(|(_, m)| {
				let end = addr + size.get() * PAGE_SIZE;
				let other_end = m.addr + m.size.get() * PAGE_SIZE;
				addr < other_end && m.addr < end
			});
			if unlikely(used) {
				return Err(errno!(EEXIST));
			}
			remove_gaps_in_range(transaction, addr, size.get())?;
			Ok(MemMapping::new(addr, size, prot, flags, file, off)?)
		} else if flags & MAP_FIXED != 0 {
			Self::unmap_impl(transaction, addr, size, true)?;
			remove_gaps_in_range(transaction, addr, size.get())?;
			Ok(MemMapping::new(addr, size, prot, flags, file, off)?)
		} else {
			// Use the address as a hint
			let (gap, gap_off) = transaction
				.state
				// Get the gap for the address. If NULL, this should fail
				.get_gap_for_addr(addr)
				.and_then(|gap| {
					// Offset in the gap
					let off = gap.get_page_offset_for(addr);
					// Check whether the mapping fits in the gap
					let end = off.checked_add(size.get())?;
					(end <= gap.get_size().get()).then_some((gap.clone(), off))
				})
				// If the hint cannot be satisfied, get a large enough gap somewhere else
				.or_else(|| {
					let gap = transaction.state.get_gap(size)?;
					// Put at the end of the gap the minimize the likelihood of colliding with
					// `brk`
					let off = gap.get_size().get() - size.get();
					Some((gap.clone(), off))
				})
				.ok_or(AllocError)?;
			// Split the old gap to fit the mapping, and insert new gaps
			let (left_gap, right_gap) = gap.consume(gap_off, size.get());
			transaction.remove_gap(gap.get_begin())?;
			if let Some(new_gap) = left_gap {
				transaction.insert_gap(new_gap)?;
			}
			if let Some(new_gap) = right_gap {
				transaction.insert_gap(new_gap)?;
			}
			let addr = gap.get_begin() + gap_off * PAGE_SIZE;
			Ok(MemMapping::new(addr, size, prot, flags, file, off)?)
		}
	}

	/// Maps a chunk of memory.
	///
	/// The function has complexity `O(log n)`.
	///
	/// Arguments:
	/// - `map_constraint` is the constraint to fulfill for the allocation
	/// - `size` is the size of the mapping in number of memory pages
	/// - `prot` is the memory protection
	/// - `flags` is the flags for the mapping
	/// - `file` is the open file the mapping points to. If `None`, no file is mapped
	/// - `off` is the offset in `file`, if applicable
	///
	/// The underlying physical memory is not allocated directly but only when an attempt to write
	/// the memory is detected.
	///
	/// On success, the function returns a pointer to the newly mapped virtual memory.
	///
	/// If the given pointer is not page-aligned, the function returns an error.
	pub fn map(
		&self,
		addr: VirtAddr,
		size: NonZeroUsize,
		prot: u8,
		flags: i32,
		file: Option<Arc<File>>,
		off: u64,
	) -> EResult<VirtAddr> {
		let mut transaction = MemSpaceTransaction::new(self);
		let map = Self::map_impl(&mut transaction, addr, size, prot, flags, file, off)?;
		let addr = map.addr;
		transaction.insert_mapping(map)?;
		transaction.commit();
		Ok(addr)
	}

	/// Maps a chunk of memory population with the given static pages.
	pub fn map_special(&self, prot: u8, flags: i32, pages: &[RcFrame]) -> AllocResult<VirtAddr> {
		let Some(len) = NonZeroUsize::new(pages.len()) else {
			return Err(AllocError);
		};
		let mut transaction = MemSpaceTransaction::new(self);
		let map = Self::map_impl(
			&mut transaction,
			VirtAddr::default(),
			len,
			prot,
			flags,
			None,
			0,
		)
		.map_err(|_| AllocError)?;
		// Populate
		map.pages
			.lock()
			.iter_mut()
			.zip(pages.iter().cloned())
			.for_each(|(dst, src)| *dst = Some(MappedFrame::new(src)));
		// Commit
		let addr = map.addr;
		transaction.insert_mapping(map)?;
		transaction.commit();
		Ok(addr)
	}

	/// Tells whether memory pages in the given range are resident (won't cause disk I/O on
	/// access).
	///
	/// Arguments:
	/// - `addr` is the starting address of the range
	/// - `size` is the number of pages, and bytes in `res`
	/// - `res` is a bitmap in which the result is written
	pub fn mincore(&self, addr: VirtAddr, size: usize, res: UserSlice<u8>) -> EResult<()> {
		let state = self.state.read();
		for i in 0..size {
			let addr = addr + size * PAGE_SIZE;
			let resident = state
				.get_mapping_for_addr(addr)
				.map(|mapping| {
					let page_offset = (addr.0 - mapping.addr.0) / PAGE_SIZE;
					let pages = mapping.pages.lock();
					let page = pages.get(page_offset);
					matches!(page, Some(Some(_)))
				})
				.unwrap_or(false);
			res.copy_to_user(i, &[resident as u8])?;
		}
		Ok(())
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
	) -> EResult<()> {
		// Remove every mapping in the chunk to unmap
		let mut i = 0;
		while i < size.get() {
			// The current page's beginning
			let page_addr = addr + i * PAGE_SIZE;
			// The mapping containing the page
			let Some(mapping) = transaction.state.get_mapping_for_addr(page_addr) else {
				// TODO jump to next mapping directly using binary tree (currently O(n log n))
				i += 1;
				continue;
			};
			// The pointer to the beginning of the mapping
			let mapping_begin = mapping.addr;
			// The offset in the mapping to the beginning of pages to unmap
			let inner_off = (page_addr.0 - mapping_begin.0) / PAGE_SIZE;
			// The number of pages to unmap in the mapping
			let pages = min(size.get() - i, mapping.size.get() - inner_off);
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
						transaction.state.get_gap_for_addr(prev_gap_ptr)
					})
					.flatten()
					.cloned();
				if let Some(p) = prev_gap {
					transaction.remove_gap(p.get_begin())?;
					gap.merge(&p);
				}
				// Merge next gap
				let next_gap = transaction.state.get_gap_for_addr(gap.get_end()).cloned();
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
	///
	/// The function frees the physical memory the mapping points to
	/// unless shared by one or several other memory mappings.
	///
	/// After this function returns, the access to the mapping of memory shall
	/// be revoked and further attempts to access it shall result in a page
	/// fault.
	#[allow(clippy::not_unsafe_ptr_arg_deref)]
	pub fn unmap(&self, addr: VirtAddr, size: NonZeroUsize) -> EResult<()> {
		// Validation
		if unlikely(!addr.is_aligned_to(PAGE_SIZE)) {
			return Err(errno!(ENOMEM));
		}
		let mut transaction = MemSpaceTransaction::new(self);
		Self::unmap_impl(&mut transaction, addr, size, false)?;
		transaction.commit();
		Ok(())
	}

	/// Binds the memory space to the current CPU.
	pub fn bind(this: &Arc<Self>) {
		if this.vmem.is_bound() {
			return;
		}
		critical(|| {
			// Mark the memory space as bound
			let prev = per_cpu().mem_space.replace(Some(this.clone()));
			// Update new bitmap
			let core_id = core_id() as usize;
			this.bound_cpus.set_bit(core_id);
			// Do actual bind
			this.vmem.bind();
			// Update old bitmap if any
			if let Some(prev) = prev {
				prev.bound_cpus.clear_bit(core_id);
			}
		});
	}

	/// Unbinds the current memory space, binding the kernel's virtual memory context instead.
	pub fn unbind() {
		critical(|| {
			// Update per-CPU structure
			let prev = per_cpu().mem_space.replace(None);
			// Bind the kernel's vmem
			KERNEL_VMEM.bind();
			// Update old bitmap if any
			if let Some(prev) = prev {
				let core_id = core_id() as usize;
				prev.bound_cpus.clear_bit(core_id);
			}
		});
	}

	/// Returns an iterator over the IDs of CPUs bounding the memory space.
	pub fn bound_cpus(&self) -> impl Iterator<Item = u32> {
		self.bound_cpus
			.iter()
			.enumerate()
			.filter(|(_, b)| *b)
			.map(|(i, _)| i as _)
	}

	/// Temporarily switches to `this` to executes the closure `f`.
	///
	/// After execution, the function restores the previous memory space.
	///
	/// `f` is executed in a critical section to prevent the temporary memory space from being
	/// replaced by the scheduler.
	pub fn switch<F: FnOnce(&Arc<Self>) -> T, T>(this: &Arc<Self>, f: F) -> T {
		critical(|| {
			let old = per_cpu().mem_space.get();
			Self::bind(this);
			let res = f(this);
			match old {
				Some(old) => MemSpace::bind(&old),
				None => MemSpace::unbind(),
			}
			res
		})
	}

	/// Clones the current memory space for process forking.
	pub fn fork(&self) -> AllocResult<MemSpace> {
		let bound_cpus = cpu::Bitmap::new(false)?;
		// Lock
		let state = self.state.read();
		// Clone first to mark as shared
		let mappings = state.mappings.try_clone()?;
		// Unmap to invalidate the virtual memory context
		for (_, m) in &state.mappings {
			if m.prot & PROT_WRITE != 0 {
				self.vmem.unmap_range(m.addr, m.size.get());
				shootdown_range(m.addr, m.size.get(), self.bound_cpus());
			}
		}
		Ok(Self {
			state: IntRwLock::new(MemSpaceState {
				gaps: state.gaps.try_clone()?,
				mappings,

				brk_init: state.brk_init,
				brk: state.brk,

				vmem_usage: state.vmem_usage,
			}),
			vmem: unsafe { VMem::new() },

			exe_info: self.exe_info.clone(),

			bound_cpus,
		})
	}

	/// Sets protection for the given range of memory.
	///
	/// Arguments:
	/// - `addr` is the address to the beginning of the range to be set
	/// - `len` is the length of the range in bytes
	/// - `prot` is a set of mapping flags
	///
	/// If a mapping to be modified is associated with a file, and the file doesn't have the
	/// matching permissions, the function returns an error.
	pub fn set_prot(&self, _addr: *mut c_void, _len: usize, _prot: u8) -> EResult<()> {
		// TODO Iterate on mappings in the range:
		//		If the mapping is shared and associated to a file, check file permissions match
		// `prot` (only write)
		//		Split the mapping if needed
		//		Set permissions
		//		Update vmem
		Ok(())
	}

	/// Performs the `brk` system call.
	///
	/// On failure, the function does nothing and returns the current brk address.
	#[allow(clippy::not_unsafe_ptr_arg_deref)]
	pub fn brk(&self, addr: VirtAddr) -> VirtAddr {
		let mut transaction = MemSpaceTransaction::new(self);
		let old = transaction.state.brk;
		if addr >= old {
			// Allocate memory
			let begin = old.align_to(PAGE_SIZE);
			let pages = (addr.0 - begin.0).div_ceil(PAGE_SIZE);
			let Some(pages) = NonZeroUsize::new(pages) else {
				return old;
			};
			let res = Self::map_impl(
				&mut transaction,
				begin,
				pages,
				PROT_READ | PROT_WRITE | PROT_EXEC,
				MAP_PRIVATE | MAP_FIXED_NOREPLACE | MAP_ANONYMOUS,
				None,
				0,
			)
			.and_then(|map| Ok(transaction.insert_mapping(map)?));
			if res.is_err() {
				return old;
			}
		} else {
			// Check the pointer is valid
			if unlikely(addr < transaction.state.brk_init) {
				return old;
			}
			// Free memory
			let begin = addr.align_to(PAGE_SIZE);
			let pages = (begin.0 - addr.0).div_ceil(PAGE_SIZE);
			let Some(pages) = NonZeroUsize::new(pages) else {
				return old;
			};
			let res = Self::unmap_impl(&mut transaction, begin, pages, true);
			if res.is_err() {
				return old;
			}
		}
		transaction.state.brk = addr;
		transaction.commit();
		addr
	}

	/// Synchronizes memory to the backing storage on the given range.
	///
	/// Arguments:
	/// - `addr` is the address to the beginning of the range
	/// - `pages` is the number of pages in the range
	/// - `sync` tells whether the synchronization should be performed synchronously
	pub fn sync(&self, addr: VirtAddr, pages: usize, sync: bool) -> EResult<()> {
		let state = self.state.read();
		// Iterate over mappings
		let mut i = 0;
		while i < pages {
			let mapping = state.get_mapping_for_addr(addr).ok_or(AllocError)?;
			mapping.sync(&self.vmem, sync)?;
			i += mapping.size.get();
		}
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
	pub fn handle_page_fault(&self, addr: VirtAddr, code: u32) -> EResult<bool> {
		let state = self.state.read();
		let Some(mapping) = state.get_mapping_for_addr(addr) else {
			return Ok(false);
		};
		// Check permissions
		let write = code & PAGE_FAULT_WRITE != 0;
		if unlikely(write && mapping.prot & PROT_WRITE == 0 && x86::is_write_protected()) {
			return Ok(false);
		}
		if unlikely(code & PAGE_FAULT_INSTRUCTION != 0 && mapping.prot & PROT_EXEC == 0) {
			return Ok(false);
		}
		// Map the accessed page
		let page_offset = (addr.0 - mapping.addr.0) / PAGE_SIZE;
		mapping.map(self, page_offset, write)?;
		Ok(true)
	}
}

impl fmt::Debug for MemSpace {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let state = self.state.read();
		fmt::Debug::fmt(&*state, f)
	}
}

impl Drop for MemSpace {
	fn drop(&mut self) {
		let mut state = self.state.write();
		// Synchronize all mappings to disk
		let mappings = mem::take(&mut state.mappings);
		for (_, m) in mappings {
			// Ignore I/O errors
			let _ = m.sync(&self.vmem, true);
		}
	}
}
