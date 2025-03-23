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
	memory::{
		buddy::ZONE_KERNEL,
		cache::{FrameOwner, RcFrame},
		VirtAddr,
	},
	process::mem_space::{MemSpace, Page, MAP_ANONYMOUS, MAP_PRIVATE, PROT_EXEC, PROT_READ},
	sync::once::OnceInit,
};
use core::{cmp::min, num::NonZeroUsize, ptr::NonNull};
use utils::{
	collections::vec::Vec,
	errno::{AllocResult, CollectResult, EResult},
	include_bytes_aligned,
	limits::PAGE_SIZE,
};

/// Information on the vDSO ELF image.
struct Vdso {
	/// The list of pages on which the image is loaded.
	pages: Vec<RcFrame>,
	/// The offset of the vDSO's entry.
	entry_off: Option<NonZeroUsize>,
}

/// Information about the mapped vDSO.
pub struct MappedVDSO {
	/// The virtual address to the beginning of the vDSO
	pub begin: VirtAddr,
	/// The pointer to the entry point of the vDSO
	pub entry: Option<NonNull<u8>>,
}

/// The info of the vDSO. If `None`, the vDSO is not loaded yet.
static VDSO: OnceInit<Vdso> = unsafe { OnceInit::new() };
/// Same as [`VDSO`], except for the compat image.
#[cfg(target_arch = "x86_64")]
static VDSO_COMPAT: OnceInit<Vdso> = unsafe { OnceInit::new() };

/// Loads the vDSO in memory and returns the image.
fn load_image(elf: &[u8]) -> EResult<Vdso> {
	let parser = ELFParser::new(elf)?;
	// Load image into pages
	let pages_count = elf.len().div_ceil(PAGE_SIZE);
	let pages = (0..pages_count)
		.map(|i| {
			let off = i * PAGE_SIZE;
			let len = min(PAGE_SIZE, elf.len() - off);
			// Alloc page
			let page = RcFrame::new(0, ZONE_KERNEL, FrameOwner::Anon, 0)?;
			let virtaddr = unsafe { &mut *page.virt_addr().as_ptr::<Page>() };
			// Copy data
			let src = &elf[off..(off + len)];
			virtaddr[..src.len()].copy_from_slice(src);
			virtaddr[src.len()..].fill(0);
			Ok(page)
		})
		.collect::<AllocResult<CollectResult<_>>>()?
		.0?;
	Ok(Vdso {
		pages,
		entry_off: NonZeroUsize::new(parser.hdr().e_entry as usize),
	})
}

/// Maps the vDSO into the given memory space.
///
/// If `compat` is true, the compatibility image is used.
///
/// The function returns the virtual address to the mapped vDSO.
#[allow(unused_variables)]
pub fn map(mem_space: &mut MemSpace, compat: bool) -> EResult<MappedVDSO> {
	#[cfg(not(target_arch = "x86_64"))]
	let vdso = &*VDSO;
	#[cfg(target_arch = "x86_64")]
	let vdso = {
		if !compat {
			&*VDSO
		} else {
			&*VDSO_COMPAT
		}
	};
	let begin = mem_space.map_special(
		PROT_READ | PROT_EXEC,
		MAP_PRIVATE | MAP_ANONYMOUS,
		&vdso.pages,
	)?;
	Ok(MappedVDSO {
		begin: begin.into(),
		entry: vdso
			.entry_off
			.and_then(|off| NonNull::new(begin.wrapping_add(off.get()))),
	})
}

/// Loads the vDSO.
pub(crate) fn init() -> EResult<()> {
	// Main image
	unsafe {
		static ELF: &[u8] = include_bytes_aligned!(usize, env!("VDSO_PATH"));
		OnceInit::init(&VDSO, load_image(ELF)?);
	}
	// 32 bit image for backward compat
	#[cfg(target_arch = "x86_64")]
	unsafe {
		static ELF: &[u8] = include_bytes_aligned!(usize, env!("VDSO_COMPAT_PATH"));
		OnceInit::init(&VDSO_COMPAT, load_image(ELF)?);
	}
	Ok(())
}
