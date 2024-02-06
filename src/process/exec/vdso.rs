//! The vDSO (virtual dynamic shared object) is a small shared library that the kernel
//! automatically maps into the memory space of all userspace programs.

use crate::{
	elf::parser::ELFParser,
	errno::Errno,
	include_bytes_aligned, memory,
	memory::buddy,
	process::{
		mem_space,
		mem_space::{MapConstraint, MapResidence, MemSpace},
	},
	util::{container::vec::Vec, lock::Mutex, math, ptr::arc::Arc},
};
use core::{cmp::min, ffi::c_void, num::NonZeroUsize, ptr, ptr::NonNull};

/// The ELF image of the vDSO.
static ELF_IMAGE: &[u8] = include_bytes_aligned!(usize, env!("VDSO_PATH"));

/// Informations on the vDSO ELF image.
struct Vdso {
	/// The list of pages on which the image is loaded.
	pages: Arc<Vec<NonNull<[u8; memory::PAGE_SIZE]>>>,
	/// The length of the ELF image in bytes.
	len: usize,

	/// The offset of the vDSO's entry.
	entry_off: usize,
}

/// Informations about mapped vDSO.
pub struct MappedVDSO {
	/// The virtual address to the beginning of the vDSO.
	pub ptr: *mut c_void,
	/// The virtual pointer to the entry point of the vDSO.
	pub entry: NonNull<c_void>,
}

/// The info of the vDSO. If `None`, the vDSO is not loaded yet.
static VDSO: Mutex<Option<Vdso>> = Mutex::new(None);

/// Loads the vDSO in memory and returns the image.
fn load_image() -> Result<Vdso, Errno> {
	let parser = ELFParser::new(ELF_IMAGE)?;
	let entry_off = parser.hdr().e_entry as _;

	// Load image into pages
	// TODO collect
	let mut pages = Vec::new();
	for i in 0..math::ceil_div(ELF_IMAGE.len(), memory::PAGE_SIZE) {
		let off = i * memory::PAGE_SIZE;
		let len = min(memory::PAGE_SIZE, ELF_IMAGE.len() - off);
		let ptr = unsafe {
			// Alloc page
			let mut ptr = buddy::alloc(0, buddy::FLAG_ZONE_TYPE_KERNEL)?;
			let virt_ptr = memory::kern_to_virt(ptr.as_mut()) as _;
			// Copy data
			ptr::copy_nonoverlapping(ELF_IMAGE[off..].as_ptr() as *const c_void, virt_ptr, len);
			ptr
		};
		pages.push(ptr.cast())?;
	}

	Ok(Vdso {
		pages: Arc::new(pages)?,
		len: ELF_IMAGE.len(),

		entry_off,
	})
}

/// Maps the vDSO into the given memory space.
///
/// The function returns the virtual pointer to the mapped vDSO.
pub fn map(mem_space: &mut MemSpace) -> Result<MappedVDSO, Errno> {
	// TODO cleanup
	let mut elf_image = VDSO.lock();
	if elf_image.is_none() {
		let img = load_image().expect("Failed to load vDSO");
		*elf_image = Some(img);
	}
	let img = elf_image.as_ref().unwrap();

	let vdso_pages = math::ceil_div(img.len, memory::PAGE_SIZE);
	let Some(vdso_pages) = NonZeroUsize::new(vdso_pages) else {
		panic!("Invalid vDSO image");
	};
	// TODO ASLR
	let ptr = mem_space.map(
		MapConstraint::None,
		vdso_pages,
		mem_space::MAPPING_FLAG_USER,
		MapResidence::Static {
			pages: img.pages.clone(),
		},
	)?;

	let entry = NonNull::new(unsafe { ptr.add(img.entry_off) }).unwrap();

	Ok(MappedVDSO {
		ptr,
		entry,
	})
}
