//! The vDSO (virtual dynamic shared object) is a small shared library that the kernel
//! automatically maps into the memory space of all userspace programs.

use core::cmp::min;
use core::ffi::c_void;
use core::ptr::NonNull;
use core::ptr;
use crate::elf::parser::ELFParser;
use crate::errno::Errno;
use crate::memory::buddy;
use crate::memory;
use crate::process::mem_space::MapConstraint;
use crate::process::mem_space::MapResidence;
use crate::process::mem_space::MemSpace;
use crate::process::mem_space;
use crate::util::container::vec::Vec;
use crate::util::lock::Mutex;
use crate::util::math;
use crate::util::ptr::SharedPtr;

/// Structure storing informations on the vDSO ELF image.
struct VDSO {
	/// The list of pages on which the image is loaded.
	pages: SharedPtr<Vec<NonNull<[u8; memory::PAGE_SIZE]>>>,
	/// The length of the ELF image in bytes.
	len: usize,

	/// The offset of the vDSO's entry.
	entry_off: usize,
}

/// Informations about mapped vDSO.
pub struct MappedVDSO {
	/// The virtual address to the beginning of the vDSO.
	pub ptr: NonNull<c_void>,
	/// The virtual pointer to the entry point of the vDSO.
	pub entry: NonNull<c_void>,
}

/// The info of the vDSO. If None, the vDSO is not loaded yet.
static ELF_IMAGE: Mutex<Option<VDSO>> = Mutex::new(None);

/// TODO doc
fn load_image() -> Result<VDSO, Errno> {
	let const_img = include_bytes!("../../../vdso.so");

	let parser = ELFParser::new(const_img)?;
	let entry_off = parser.get_header().e_entry as _;

	// Load image into pages
	let mut pages = Vec::new();
	for i in 0..math::ceil_div(const_img.len(), memory::PAGE_SIZE) {
		// Alloc page
		let ptr = buddy::alloc(0, buddy::FLAG_ZONE_TYPE_KERNEL)?;
		let virt_ptr = memory::kern_to_virt(ptr) as _;

		// Copy data
		let off = i * memory::PAGE_SIZE;
		let len = min(memory::PAGE_SIZE, const_img.len() - off);
		unsafe {
			ptr::copy_nonoverlapping(const_img[off..].as_ptr() as *const c_void, virt_ptr, len);
		}

		pages.push(NonNull::new(ptr as *mut [u8; memory::PAGE_SIZE]).unwrap())?;
	}

	Ok(VDSO {
		pages: SharedPtr::new(pages)?,
		len: const_img.len(),

		entry_off,
	})
}

/// Maps the vDSO into the given memory space.
///
/// The function returns the virtual pointer to the mapped vDSO.
pub fn map(mem_space: &mut MemSpace) -> Result<MappedVDSO, Errno> {
	let elf_image_guard = ELF_IMAGE.lock();
	let elf_image = elf_image_guard.get_mut();

	if elf_image.is_none() {
		let img = load_image().expect("Failed to load vDSO");
		*elf_image = Some(img);
	}
	let img = elf_image.as_ref().unwrap();

	// TODO ASLR
	let ptr = mem_space.map(
		MapConstraint::None,
		math::ceil_div(img.len, memory::PAGE_SIZE),
		mem_space::MAPPING_FLAG_USER,
		MapResidence::Static {
			pages: img.pages.clone(),
		}
	)?;

	let entry = unsafe {
		ptr.add(img.entry_off)
	};

	Ok(MappedVDSO {
		ptr: NonNull::new(ptr).unwrap(),
		entry: NonNull::new(entry).unwrap(),
	})
}
