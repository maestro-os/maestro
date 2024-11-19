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

//! Implementation of ELF programs execution with respect to the **System V ABI**.

use super::vdso;
use crate::{
	arch::x86,
	elf,
	elf::{
		parser::{Class, ELFParser, ProgramHeader, Rel, Rela},
		relocation,
		relocation::GOT_SYM,
	},
	file::{perm::AccessProfile, vfs, FileType},
	memory::{vmem, VirtAddr},
	process,
	process::{
		exec::{vdso::MappedVDSO, ExecInfo, Executor, ProgramImage},
		mem_space,
		mem_space::{residence::MapResidence, MapConstraint, MemSpace},
	},
};
use core::{
	cmp::{max, min},
	intrinsics::unlikely,
	mem::size_of,
	num::NonZeroUsize,
	ptr,
	ptr::null_mut,
	slice,
};
use utils::{
	collections::{string::String, vec::Vec},
	errno,
	errno::EResult,
	limits::PAGE_SIZE,
	ptr::arc::Arc,
};

/// Used to define the end of the entries list.
const AT_NULL: i32 = 0;
/// Entry with no meaning, to be ignored.
const AT_IGNORE: i32 = 1;
/// Entry containing a file descriptor to the application object file in case
/// the program is run using an interpreter.
const AT_EXECFD: i32 = 2;
/// Entry containing a pointer to the program header table for the interpreter.
const AT_PHDR: i32 = 3;
/// The size in bytes of one entry in the program header table to which AT_PHDR
/// points.
const AT_PHENT: i32 = 4;
/// The number of entries in the program header table to which AT_PHDR points.
const AT_PHNUM: i32 = 5;
/// The system's page size in bytes.
const AT_PAGESZ: i32 = 6;
/// The base address at which the interpreter program was loaded in memory.
const AT_BASE: i32 = 7;
/// Contains flags.
const AT_FLAGS: i32 = 8;
/// Entry with the pointer to the entry point of the program to which the
/// interpreter should transfer control.
const AT_ENTRY: i32 = 9;
/// A boolean value. If non-zero, the program is non-ELF.
const AT_NOTELF: i32 = 10;
/// The real user ID of the process.
const AT_UID: i32 = 11;
/// The effective user ID of the process.
const AT_EUID: i32 = 12;
/// The real group ID of the process.
const AT_GID: i32 = 13;
/// The effective group ID of the process.
const AT_EGID: i32 = 14;
/// Entry pointing to a string containing the platform name.
const AT_PLATFORM: i32 = 15;
/// A bitmask of CPU features. Equivalent to the value returned by CPUID 1.EDX.
const AT_HWCAP: i32 = 16;
/// The frequency at which times() increments.
const AT_CLKTCK: i32 = 17;
/// A boolean value. If non-zero, the program is started in secure mode (suid).
const AT_SECURE: i32 = 23;
/// Entry pointing to a string containing the base platform name.
const AT_BASE_PLATFORM: i32 = 24;
/// Points to 16 randomly generated secure bytes.
const AT_RANDOM: i32 = 25;
/// Extended hardware feature mask.
const AT_HWCAP2: i32 = 26;
/// A pointer to the filename of the executed program.
const AT_EXECFN: i32 = 31;
/// A pointer to the entry point of the vDSO.
const AT_SYSINFO: i32 = 32;
/// A pointer to the beginning of the vDSO ELF image.
const AT_SYSINFO_EHDR: i32 = 33;

/// Information returned after loading an ELF program used to finish
/// initialization.
#[derive(Debug)]
struct ELFLoadInfo {
	/// The pointer to the end of loaded segments
	load_end: *mut u8,

	/// The pointer to the program header if present
	phdr: VirtAddr,
	/// The length in bytes of an entry in the program headers table.
	phentsize: usize,
	/// The number of entries in the program headers table.
	phnum: usize,

	/// The pointer to the entry point
	entry_point: VirtAddr,
}

/// An entry of System V's Auxiliary Vectors.
#[repr(C)]
struct AuxEntry {
	/// The entry's type.
	a_type: i32,
	/// The entry's value.
	a_val: isize,
}

/// Enumeration of possible values for an auxiliary vector entry.
enum AuxEntryDescValue {
	/// A single number.
	Number(usize),
	/// A string of bytes.
	String(&'static [u8]),
}

/// Structure describing an auxiliary vector entry.
struct AuxEntryDesc {
	/// The entry's type.
	a_type: i32,
	/// The entry's value.
	a_val: AuxEntryDescValue,
}

impl AuxEntryDesc {
	/// Creates a new instance with the given type `a_type` and value `a_val`.
	pub fn new(a_type: i32, a_val: AuxEntryDescValue) -> Self {
		Self {
			a_type,
			a_val,
		}
	}
}

/// Builds an auxiliary vector.
///
/// Arguments:
/// - `exec_info` is the set of execution information.
/// - `load_info` is the set of ELF load information.
/// - `vdso` is the set of vDSO information.
fn build_auxiliary(
	exec_info: &ExecInfo,
	load_info: &ELFLoadInfo,
	vdso: &MappedVDSO,
) -> EResult<Vec<AuxEntryDesc>> {
	let mut aux = Vec::new();

	aux.push(AuxEntryDesc::new(
		AT_PHDR,
		AuxEntryDescValue::Number(load_info.phdr.0),
	))?;
	aux.push(AuxEntryDesc::new(
		AT_PHENT,
		AuxEntryDescValue::Number(load_info.phentsize as _),
	))?;
	aux.push(AuxEntryDesc::new(
		AT_PHNUM,
		AuxEntryDescValue::Number(load_info.phnum as _),
	))?;

	aux.push(AuxEntryDesc::new(
		AT_PAGESZ,
		AuxEntryDescValue::Number(PAGE_SIZE),
	))?;

	aux.push(AuxEntryDesc::new(AT_NOTELF, AuxEntryDescValue::Number(0)))?;
	aux.push(AuxEntryDesc::new(
		AT_UID,
		AuxEntryDescValue::Number(exec_info.path_resolution.access_profile.uid as _),
	))?;
	aux.push(AuxEntryDesc::new(
		AT_EUID,
		AuxEntryDescValue::Number(exec_info.path_resolution.access_profile.euid as _),
	))?;
	aux.push(AuxEntryDesc::new(
		AT_GID,
		AuxEntryDescValue::Number(exec_info.path_resolution.access_profile.gid as _),
	))?;
	aux.push(AuxEntryDesc::new(
		AT_EGID,
		AuxEntryDescValue::Number(exec_info.path_resolution.access_profile.egid as _),
	))?;
	aux.push(AuxEntryDesc::new(
		AT_PLATFORM,
		AuxEntryDescValue::String(crate::NAME.as_bytes()),
	))?;

	let hwcap = x86::get_hwcap();
	aux.push(AuxEntryDesc::new(
		AT_HWCAP,
		AuxEntryDescValue::Number(hwcap as _),
	))?;

	aux.push(AuxEntryDesc::new(AT_SECURE, AuxEntryDescValue::Number(0)))?; // TODO
	aux.push(AuxEntryDesc::new(
		AT_BASE_PLATFORM,
		AuxEntryDescValue::String(crate::NAME.as_bytes()),
	))?;
	aux.push(AuxEntryDesc::new(
		AT_RANDOM,
		AuxEntryDescValue::String(&[0; 16]),
	))?; // TODO
	aux.push(AuxEntryDesc::new(
		AT_EXECFN,
		AuxEntryDescValue::String("TODO\0".as_bytes()),
	))?; // TODO

	// vDSO
	aux.push(AuxEntryDesc::new(
		AT_SYSINFO,
		AuxEntryDescValue::Number(vdso.entry.as_ptr() as _),
	))?;
	aux.push(AuxEntryDesc::new(
		AT_SYSINFO_EHDR,
		AuxEntryDescValue::Number(vdso.begin.0),
	))?;

	// End
	aux.push(AuxEntryDesc::new(AT_NULL, AuxEntryDescValue::Number(0)))?;

	Ok(aux)
}

/// Reads the file `file`.
///
/// `ap` is the access profile to check permissions.
///
/// If the file is not executable, the function returns an error.
fn read_exec_file(file: &vfs::Entry, ap: &AccessProfile) -> EResult<Vec<u8>> {
	// Check that the file can be executed by the user
	let stat = file.stat()?;
	if unlikely(stat.get_type() != Some(FileType::Regular)) {
		return Err(errno!(EACCES));
	}
	if unlikely(!ap.can_execute_file(&stat)) {
		return Err(errno!(EACCES));
	}
	file.read_all()
}

/// Allocates memory in userspace for an ELF segment.
///
/// If the segment is not loadable, the function does nothing.
///
/// Arguments:
/// - `load_base` is the address at which the executable is loaded.
/// - `mem_space` is the memory space to allocate into.
/// - `seg` is the segment for which the memory is allocated.
///
/// If loaded, the function return the pointer to the end of the segment in
/// virtual memory.
fn alloc_segment(
	load_base: *mut u8,
	mem_space: &mut MemSpace,
	seg: &ProgramHeader,
) -> EResult<Option<*mut u8>> {
	// Load only loadable segments
	if seg.p_type != elf::PT_LOAD && seg.p_type != elf::PT_PHDR {
		return Ok(None);
	}
	// Check the alignment is correct
	if unlikely(!seg.p_align.is_power_of_two()) {
		return Err(errno!(EINVAL));
	}
	// The size of the padding before the segment
	let pad = seg.p_vaddr as usize % max(seg.p_align as usize, PAGE_SIZE);
	// The pointer to the beginning of the segment in memory
	let mem_begin = load_base.wrapping_add(seg.p_vaddr as usize - pad);
	// The length of the memory to allocate in pages
	let pages = (pad + seg.p_memsz as usize).div_ceil(PAGE_SIZE);
	if let Some(pages) = NonZeroUsize::new(pages) {
		mem_space.map(
			MapConstraint::Fixed(VirtAddr::from(mem_begin)),
			pages,
			seg.get_mem_space_flags(),
			MapResidence::Normal,
		)?;
		// Pre-allocate the pages to make them writable
		mem_space.alloc(VirtAddr::from(mem_begin), pages.get() * PAGE_SIZE)?;
	}
	// The pointer to the end of the virtual memory chunk
	let mem_end = mem_begin.wrapping_add(pages * PAGE_SIZE);
	Ok(Some(mem_end))
}

/// Copies the segment's data into memory.
///
/// If the segment isn't loadable, the function does nothing.
///
/// Arguments:
/// - `load_base` is the address at which the executable is loaded.
/// - `seg` is the segment.
/// - `image` is the ELF file image.
fn copy_segment(load_base: *mut u8, seg: &ProgramHeader, image: &[u8]) {
	// Load only loadable segments
	if seg.p_type != elf::PT_LOAD && seg.p_type != elf::PT_PHDR {
		return;
	}
	// The pointer to the beginning of the segment's data in the file
	let file_begin = &image[seg.p_offset as usize];
	// The pointer to the beginning of the segment in the virtual memory
	let begin = load_base.wrapping_add(seg.p_vaddr as usize);
	// The length of data to be copied from file
	let len = min(seg.p_memsz, seg.p_filesz) as usize;
	// Copy the segment's data
	unsafe {
		vmem::write_ro(|| vmem::smap_disable(|| ptr::copy_nonoverlapping(file_begin, begin, len)));
	}
}

/// Loads the ELF file parsed by `elf` into the memory space `mem_space`.
///
/// Arguments:
/// - `elf` is the ELF image.
/// - `mem_space` is the memory space.
/// - `load_base` is the base address at which the ELF is loaded.
fn load_elf(
	elf: &ELFParser,
	mem_space: &mut MemSpace,
	load_base: *mut u8,
) -> EResult<ELFLoadInfo> {
	// Allocate memory for segments
	let mut load_end = load_base;
	for seg in elf.iter_segments() {
		if let Some(end) = alloc_segment(load_base, mem_space, &seg)? {
			load_end = max(end, load_end);
		}
	}
	let ehdr = elf.hdr();
	let phentsize = ehdr.e_phentsize as usize;
	let phnum = ehdr.e_phnum as usize;
	// The size in bytes of the phdr table
	let phdr_size = phentsize * phnum;
	let phdr = elf
		.iter_segments()
		.find(|seg| seg.p_type == elf::PT_PHDR)
		.map(|seg| ptr::with_exposed_provenance_mut(seg.p_vaddr as _));
	let (phdr, phdr_needs_copy) = match phdr {
		Some(phdr) => (phdr, false),
		// Not phdr segment. Load it manually
		None => {
			let pages = phdr_size.div_ceil(PAGE_SIZE);
			let Some(pages) = NonZeroUsize::new(pages) else {
				return Err(errno!(EINVAL));
			};
			let phdr = mem_space.map(
				MapConstraint::None,
				pages,
				mem_space::MAPPING_FLAG_USER,
				MapResidence::Normal,
			)?;
			(phdr, true)
		}
	};
	// Switch to the process's vmem to write onto the virtual memory
	unsafe {
		vmem::switch(&mem_space.vmem, move || -> EResult<()> {
			// Copy segments' data
			for seg in elf.iter_segments() {
				copy_segment(load_base, &seg, elf.as_slice());
			}
			// Copy phdr's data if necessary
			if phdr_needs_copy {
				let image_phdr = &elf.as_slice()[(ehdr.e_phoff as usize)..];
				vmem::write_ro(|| {
					vmem::smap_disable(|| {
						ptr::copy_nonoverlapping::<u8>(image_phdr.as_ptr(), phdr, phdr_size);
					});
				});
			}
			// Closure returning a symbol
			let get_sym = |sym_section: u32, sym: usize| {
				let section = elf.get_section_by_index(sym_section as _)?;
				let sym = elf.get_symbol_by_index(&section, sym as _)?;
				sym.is_defined()
					.then_some(load_base as usize + sym.st_value as usize)
			};
			let got_sym = elf.get_symbol_by_name(GOT_SYM);
			for section in elf.iter_sections() {
				for rel in elf.iter_rel::<Rel>(&section) {
					relocation::perform(
						&rel,
						load_base as _,
						&section,
						get_sym,
						got_sym.as_ref(),
						true,
					)
					.map_err(|_| errno!(EINVAL))?;
				}
				for rela in elf.iter_rel::<Rela>(&section) {
					relocation::perform(
						&rela,
						load_base as _,
						&section,
						get_sym,
						got_sym.as_ref(),
						true,
					)
					.map_err(|_| errno!(EINVAL))?;
				}
			}
			Ok(())
		})?;
	}
	Ok(ELFLoadInfo {
		load_end,

		phdr: phdr.into(),
		phentsize,
		phnum,

		entry_point: VirtAddr::from(load_base) + elf.hdr().e_entry as usize,
	})
}

/// Returns two values:
/// - The size in bytes of the buffer to store the arguments and environment variables, padding
///   included.
/// - The required size in bytes for the data to be written on the stack before the program starts.
fn get_init_stack_size(argv: &[String], envp: &[String], aux: &[AuxEntryDesc]) -> (usize, usize) {
	// The size of the block storing the arguments and environment
	let mut info_block_size = 0;
	for a in aux {
		if let AuxEntryDescValue::String(slice) = a.a_val {
			info_block_size += slice.len() + 1;
		}
	}
	for e in envp {
		info_block_size += e.len() + 1;
	}
	for a in argv {
		info_block_size += a.len() + 1;
	}

	// The padding before the information block allowing to preserve stack alignment
	let info_block_pad = 4 - (info_block_size % 4);

	// The size of the auxiliary vector
	let aux_size = aux.len() * size_of::<AuxEntry>();
	// The size of the environment pointers + the null fourbyte
	let envp_size = envp.len() * 4 + 4;
	// The size of the argument pointers + the null fourbyte + argc
	let argv_size = argv.len() * 4 + 8;

	// The total size of the stack data in bytes
	let total_size = info_block_size + info_block_pad + aux_size + envp_size + argv_size;

	(info_block_size + info_block_pad, total_size)
}

/// Helper to pre-allocate space on the stack.
///
/// `len` is the space to allocate in bytes.
fn stack_prealloc(mem_space: &mut MemSpace, stack: *mut u8, len: usize) -> EResult<()> {
	let pages_count = len.div_ceil(PAGE_SIZE);
	if unlikely(pages_count >= process::USER_STACK_SIZE) {
		return Err(errno!(ENOMEM));
	}
	let len = pages_count * PAGE_SIZE;
	let begin = VirtAddr::from(stack) - len;
	mem_space.alloc(begin, len)?;
	Ok(())
}

/// Initializes the stack data of the process according to the System V ABI.
///
/// The start/end of `argv` and `envp` in userspace are also updated into `exe_info`.
///
/// Arguments:
/// - `user_stack` the pointer to the user stack.
/// - `argv` is the list of arguments.
/// - `envp` is the environment.
/// - `aux` is the auxiliary vector.
///
/// The function returns the distance between the top of the stack and the
/// new bottom after the data has been written.
fn init_stack(
	user_stack: *mut u8,
	argv: &[String],
	envp: &[String],
	aux: &[AuxEntryDesc],
	exe_info: &mut mem_space::ExeInfo,
) {
	let (info_size, total_size) = get_init_stack_size(argv, envp, aux);
	// A slice on the stack representing the region which will contain the
	// arguments and environment variables
	let info_slice = unsafe {
		let ptr = user_stack.sub(info_size);
		slice::from_raw_parts_mut(ptr, info_size)
	};
	// A slice on the stack representing the region to fill
	let stack_slice = unsafe {
		let ptr = user_stack.sub(total_size) as *mut u32;
		slice::from_raw_parts_mut(ptr, total_size / size_of::<u32>())
	};
	// The offset in the information block
	let mut info_off = 0;
	// The offset in the pointers list
	let mut stack_off = 0;
	// Set argc
	stack_slice[stack_off] = argv.len() as u32;
	stack_off += 1;
	// Set argv
	exe_info.argv_begin = VirtAddr::from(info_slice.as_ptr()) + info_off;
	for arg in argv {
		// Set the argument's pointer
		stack_slice[stack_off] = &info_slice[info_off] as *const _ as u32;
		// Copy string
		let len = arg.len();
		info_slice[info_off..(info_off + len)].copy_from_slice(arg);
		info_slice[info_off + len] = 0;
		info_off += len + 1;
		stack_off += 1;
	}
	// Set the nul byte to end argv
	stack_slice[stack_off] = 0;
	stack_off += 1;
	// Set environment
	exe_info.argv_end = VirtAddr::from(info_slice.as_ptr()) + info_off;
	exe_info.envp_begin = exe_info.argv_end;
	for var in envp {
		// Set the variable's pointer
		stack_slice[stack_off] = &info_slice[info_off] as *const _ as u32;
		// Copy string
		let len = var.len();
		info_slice[info_off..(info_off + len)].copy_from_slice(var);
		info_slice[info_off + len] = 0;
		info_off += len + 1;
		stack_off += 1;
	}
	// Set the nul bytes to end envp
	stack_slice[stack_off] = 0;
	stack_off += 1;
	exe_info.envp_end = VirtAddr::from(info_slice.as_ptr()) + info_off;
	// Set auxiliary vector
	for a in aux {
		let val = match a.a_val {
			AuxEntryDescValue::Number(n) => n as _,
			AuxEntryDescValue::String(slice) => {
				let val = &info_slice[info_off] as *const _ as _;
				// Copy string
				let len = slice.len();
				info_slice[info_off..(info_off + len)].copy_from_slice(slice);
				info_slice[info_off + len] = 0;
				info_off += len + 1;
				val
			}
		};
		// Set the entry
		stack_slice[stack_off] = a.a_type as _;
		stack_slice[stack_off + 1] = val;
		stack_off += 2;
	}
}

/// The program executor for ELF files.
pub struct ELFExecutor<'s>(pub ExecInfo<'s>);

impl<'s> Executor for ELFExecutor<'s> {
	// TODO Ensure there is no way to write in kernel space (check segments position
	// and relocations)
	// TODO Handle suid and sgid
	fn build_image(&self, file: Arc<vfs::Entry>) -> EResult<ProgramImage> {
		let image = read_exec_file(&file, &self.0.path_resolution.access_profile)?;
		let parser = ELFParser::new(&image)?;
		let mut mem_space = MemSpace::new(file)?;
		let load_info = load_elf(&parser, &mut mem_space, null_mut())?;
		let user_stack = mem_space
			.map(
				MapConstraint::None,
				process::USER_STACK_SIZE.try_into().unwrap(),
				process::USER_STACK_FLAGS,
				MapResidence::Normal,
			)?
			.wrapping_add(process::USER_STACK_SIZE * PAGE_SIZE);
		let vdso = vdso::map(&mut mem_space)?;
		// Initialize the userspace stack
		let aux = build_auxiliary(&self.0, &load_info, &vdso)?;
		let (_, init_stack_size) = get_init_stack_size(&self.0.argv, &self.0.envp, &aux);
		stack_prealloc(&mut mem_space, user_stack, init_stack_size)?;
		unsafe {
			vmem::switch(&mem_space.vmem, || {
				vmem::smap_disable(|| {
					init_stack(
						user_stack,
						&self.0.argv,
						&self.0.envp,
						&aux,
						&mut mem_space.exe_info,
					);
				});
			});
		}
		mem_space.set_brk_init(VirtAddr::from(load_info.load_end).align_to(PAGE_SIZE));
		Ok(ProgramImage {
			mem_space,
			bit32: parser.class() == Class::Bit32,

			entry_point: load_info.entry_point,
			user_stack: VirtAddr::from(user_stack) - init_stack_size,
		})
	}
}
