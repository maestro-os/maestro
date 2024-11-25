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

//! The vDSO (virtual dynamic shared object) is a small shared library that the kernel
//! automatically maps into the memory space of all userspace programs.

use crate::{
	elf::parser::ELFParser,
	memory::{buddy, VirtAddr},
	process::{
		mem_space,
		mem_space::{
			residence::{MapResidence, Page, ResidencePage},
			MapConstraint, MemSpace,
		},
	},
	sync::mutex::Mutex,
};
use core::{cmp::min, num::NonZeroUsize, ptr::NonNull};
use utils::{
	collections::vec::Vec,
	errno::{AllocResult, CollectResult, EResult},
	include_bytes_aligned,
	limits::PAGE_SIZE,
	ptr::arc::Arc,
};

/// The ELF image of the vDSO.
static ELF_IMAGE: &[u8] = include_bytes_aligned!(usize, env!("VDSO_PATH"));

/// Information on the vDSO ELF image.
struct Vdso {
	/// The list of pages on which the image is loaded.
	pages: Arc<Vec<Arc<ResidencePage>>>,
	/// The length of the ELF image in bytes.
	len: usize,

	/// The offset of the vDSO's entry.
	entry_off: usize,
}

/// Information about the mapped vDSO.
pub struct MappedVDSO {
	/// The virtual address to the beginning of the vDSO
	pub begin: VirtAddr,
	/// The pointer to the entry point of the vDSO
	pub entry: NonNull<u8>,
}

/// The info of the vDSO. If `None`, the vDSO is not loaded yet.
static VDSO: Mutex<Option<Vdso>> = Mutex::new(None);

/// Loads the vDSO in memory and returns the image.
fn load_image() -> EResult<Vdso> {
	let parser = ELFParser::new(ELF_IMAGE)?;
	let entry_off = parser.hdr().e_entry as _;
	// Load image into pages
	let pages_count = ELF_IMAGE.len().div_ceil(PAGE_SIZE);
	let pages = (0..pages_count)
		.map(|i| {
			let off = i * PAGE_SIZE;
			let len = min(PAGE_SIZE, ELF_IMAGE.len() - off);
			// Alloc page
			let physaddr = buddy::alloc(0, buddy::FLAG_ZONE_TYPE_KERNEL)?;
			let virtaddr = physaddr.kernel_to_virtual().unwrap();
			let virtaddr = unsafe { &mut *virtaddr.as_ptr::<Page>() };
			// Copy data
			let src = &ELF_IMAGE[off..(off + len)];
			virtaddr[..src.len()].copy_from_slice(src);
			virtaddr[src.len()..].fill(0);
			Arc::new(ResidencePage::new(physaddr))
		})
		.collect::<AllocResult<CollectResult<_>>>()?
		.0?;
	Ok(Vdso {
		pages: Arc::new(pages)?,
		len: ELF_IMAGE.len(),

		entry_off,
	})
}

/// Maps the vDSO into the given memory space.
///
/// The function returns the virtual pointer to the mapped vDSO.
pub fn map(mem_space: &mut MemSpace) -> EResult<MappedVDSO> {
	let mut elf_image = VDSO.lock();
	let img = elf_image.get_or_insert_with(|| load_image().expect("Failed to load vDSO"));
	let vdso_pages = img.len.div_ceil(PAGE_SIZE);
	let Some(vdso_pages) = NonZeroUsize::new(vdso_pages) else {
		panic!("Invalid vDSO image");
	};
	// TODO ASLR
	let begin = mem_space.map(
		MapConstraint::None,
		vdso_pages,
		mem_space::MAPPING_FLAG_USER,
		MapResidence::Static {
			pages: img.pages.clone(),
		},
	)?;
	let entry_ptr = begin.wrapping_add(img.entry_off);
	Ok(MappedVDSO {
		begin: begin.into(),
		entry: NonNull::new(entry_ptr).unwrap(),
	})
}
