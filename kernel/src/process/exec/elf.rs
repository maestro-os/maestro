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
		parser::{Class, ELFParser, ProgramHeader},
		ET_DYN,
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
	num::NonZeroUsize,
	ptr,
};
use utils::{
	collections::{string::String, vec::Vec},
	errno,
	errno::{AllocResult, EResult},
	limits::PAGE_SIZE,
	ptr::arc::Arc,
	vec,
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

/// Enumeration of possible values for an auxiliary vector entry.
enum AuxEntryDescValue {
	/// A single number.
	Number(usize),
	/// A string of bytes.
	String(&'static [u8]),
}

/// An auxiliary vector entry.
struct AuxEntryDesc {
	/// The entry's type.
	pub a_type: i32,
	/// The entry's value.
	pub a_val: AuxEntryDescValue,
}

/// Builds an auxiliary vector.
///
/// Arguments:
/// - `exec_info` is the set of execution information.
/// - `load_base` is the base address at which the ELF is loaded.
/// - `load_info` is the set of ELF load information.
/// - `vdso` is the set of vDSO information.
fn build_auxiliary(
	exec_info: &ExecInfo,
	load_base: *mut u8,
	load_info: &ELFLoadInfo,
	vdso: &MappedVDSO,
) -> AllocResult<Vec<AuxEntryDesc>> {
	let mut vec = vec![
		AuxEntryDesc {
			a_type: AT_PHDR,
			a_val: AuxEntryDescValue::Number(load_info.phdr.0),
		},
		AuxEntryDesc {
			a_type: AT_PHENT,
			a_val: AuxEntryDescValue::Number(load_info.phentsize as _),
		},
		AuxEntryDesc {
			a_type: AT_PHNUM,
			a_val: AuxEntryDescValue::Number(load_info.phnum as _),
		},
		AuxEntryDesc {
			a_type: AT_PAGESZ,
			a_val: AuxEntryDescValue::Number(PAGE_SIZE),
		},
		AuxEntryDesc {
			a_type: AT_BASE,
			a_val: AuxEntryDescValue::Number(load_base as _),
		},
		AuxEntryDesc {
			a_type: AT_NOTELF,
			a_val: AuxEntryDescValue::Number(0),
		},
		AuxEntryDesc {
			a_type: AT_UID,
			a_val: AuxEntryDescValue::Number(exec_info.path_resolution.access_profile.uid as _),
		},
		AuxEntryDesc {
			a_type: AT_EUID,
			a_val: AuxEntryDescValue::Number(exec_info.path_resolution.access_profile.euid as _),
		},
		AuxEntryDesc {
			a_type: AT_GID,
			a_val: AuxEntryDescValue::Number(exec_info.path_resolution.access_profile.gid as _),
		},
		AuxEntryDesc {
			a_type: AT_EGID,
			a_val: AuxEntryDescValue::Number(exec_info.path_resolution.access_profile.egid as _),
		},
		AuxEntryDesc {
			a_type: AT_PLATFORM,
			a_val: AuxEntryDescValue::String(crate::NAME.as_bytes()),
		},
		AuxEntryDesc {
			a_type: AT_HWCAP,
			a_val: AuxEntryDescValue::Number(x86::get_hwcap() as _),
		},
		AuxEntryDesc {
			a_type: AT_SECURE,
			a_val: AuxEntryDescValue::Number(0), // TODO
		},
		AuxEntryDesc {
			a_type: AT_BASE_PLATFORM,
			a_val: AuxEntryDescValue::String(crate::NAME.as_bytes()),
		},
		AuxEntryDesc {
			a_type: AT_RANDOM,
			a_val: AuxEntryDescValue::String(&[0; 16]), // TODO
		},
		AuxEntryDesc {
			a_type: AT_EXECFN,
			a_val: AuxEntryDescValue::String(b"TODO\0"), // TODO
		},
		AuxEntryDesc {
			a_type: AT_SYSINFO_EHDR,
			a_val: AuxEntryDescValue::Number(vdso.begin.0),
		},
	]?;
	if let Some(entry) = vdso.entry {
		vec.push(AuxEntryDesc {
			a_type: AT_SYSINFO,
			a_val: AuxEntryDescValue::Number(entry.as_ptr() as _),
		})?;
	}
	// End
	vec.push(AuxEntryDesc {
		a_type: AT_NULL,
		a_val: AuxEntryDescValue::Number(0),
	})?;
	Ok(vec)
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
		.map(|seg| VirtAddr::from(load_base) + seg.p_vaddr as usize);
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
			(VirtAddr::from(phdr), true)
		}
	};
	// Switch to the process's vmem to write onto the virtual memory
	unsafe {
		vmem::switch(&mem_space.vmem, move || {
			// Copy segments' data
			for seg in elf.iter_segments() {
				copy_segment(load_base, &seg, elf.as_slice());
			}
			// Copy phdr's data if necessary
			if phdr_needs_copy {
				let image_phdr = &elf.as_slice()[(ehdr.e_phoff as usize)..];
				vmem::write_ro(|| {
					vmem::smap_disable(|| {
						ptr::copy_nonoverlapping::<u8>(
							image_phdr.as_ptr(),
							phdr.as_ptr(),
							phdr_size,
						);
					});
				});
			}
		});
	}
	Ok(ELFLoadInfo {
		load_end,

		phdr,
		phentsize,
		phnum,

		entry_point: VirtAddr::from(load_base) + elf.hdr().e_entry as usize,
	})
}

/// Computes the size of the initial data on the stack.
///
/// `compat` indicates whether userspace runs in compatibility mode.
///
/// Returns the size of the "information" part, and the total size on the stack (including the
/// "information" part).
fn get_init_stack_size(
	argv: &[String],
	envp: &[String],
	aux: &[AuxEntryDesc],
	compat: bool,
) -> (usize, usize) {
	let size = if compat { 4 } else { 8 };
	// The size of the block storing the arguments and environment
	let info_block_size = aux
		.iter()
		.filter_map(|a| {
			if let AuxEntryDescValue::String(slice) = a.a_val {
				Some(slice.len() + 1)
			} else {
				None
			}
		})
		.chain(envp.iter().map(|e| e.len() + 1))
		.chain(argv.iter().map(|a| a.len() + 1))
		.sum::<usize>()
		// Add padding before the information block allowing to preserve stack alignment
		.next_multiple_of(size);
	// The size of the auxiliary vector
	let aux_size = aux.len() * (size * 2);
	// The size of the environment pointers + null
	let envp_size = (envp.len() + 1) * size;
	// The size of the argument pointers + null + argc
	let argv_size = (argv.len() + 2) * size;
	// The total size of the stack data in bytes
	let total_size = info_block_size + aux_size + envp_size + argv_size;
	(info_block_size, total_size)
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

/// Writes `val` on `stack`.
///
/// `compat` indicates whether userspace runs in compatibility mode.
///
/// # Safety
///
/// `stack` must be a valid pointer.
#[inline]
unsafe fn write_val(stack: &mut *mut u8, val: usize, compat: bool) {
	if compat {
		*(*stack as *mut u32) = val as u32;
		*stack = stack.add(4);
	} else {
		*(*stack as *mut u64) = val as u64;
		*stack = stack.add(8);
	}
}

/// Copies `str` to `stack` with a nul-terminating byte, and increases `stack` accordingly.
#[inline]
unsafe fn copy_string(stack: &mut *mut u8, str: &[u8]) {
	let len = str.len();
	ptr::copy_nonoverlapping(str.as_ptr(), *stack, len);
	*stack.add(len) = 0;
	*stack = stack.add(len + 1);
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
/// - `exe_info` is the execution information stored the memory space's structure.
/// - `compat` indicates whether userspace runs in compatibility mode.
///
/// # Safety
///
/// `stack` must point to a valid stack.
unsafe fn init_stack(
	user_stack: *mut u8,
	argv: &[String],
	envp: &[String],
	aux: &[AuxEntryDesc],
	exe_info: &mut mem_space::ExeInfo,
	compat: bool,
) {
	let (info_size, total_size) = get_init_stack_size(argv, envp, aux, compat);
	let mut info_ptr = user_stack.sub(info_size);
	let mut args_ptr = user_stack.sub(total_size);
	// Push argc
	write_val(&mut args_ptr, argv.len(), compat);
	// Set argv
	exe_info.argv_begin = VirtAddr::from(info_ptr);
	for arg in argv {
		write_val(&mut args_ptr, info_ptr as _, compat);
		copy_string(&mut info_ptr, arg);
	}
	// Set the nul byte to end argv
	write_val(&mut args_ptr, 0, compat);
	exe_info.argv_end = VirtAddr::from(info_ptr);
	// Set environment
	exe_info.envp_begin = exe_info.argv_end;
	for var in envp {
		write_val(&mut args_ptr, info_ptr as _, compat);
		copy_string(&mut info_ptr, var);
	}
	// Set the nul bytes to end envp
	write_val(&mut args_ptr, 0, compat);
	exe_info.envp_end = VirtAddr::from(info_ptr);
	// Set auxiliary vector
	for a in aux {
		let val = match a.a_val {
			AuxEntryDescValue::Number(n) => n,
			AuxEntryDescValue::String(slice) => {
				let begin = info_ptr;
				copy_string(&mut info_ptr, slice);
				begin as usize
			}
		};
		write_val(&mut args_ptr, a.a_type as _, compat);
		write_val(&mut args_ptr, val, compat);
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
		let compat = parser.class() == Class::Bit32;
		let mut mem_space = MemSpace::new(file)?;
		let load_base = if parser.hdr().e_type == ET_DYN {
			// TODO ASLR
			PAGE_SIZE
		} else {
			0
		};
		let load_base = VirtAddr(load_base).as_ptr();
		let load_info = load_elf(&parser, &mut mem_space, load_base)?;
		let user_stack = mem_space
			.map(
				MapConstraint::None,
				process::USER_STACK_SIZE.try_into().unwrap(),
				process::USER_STACK_FLAGS,
				MapResidence::Normal,
			)?
			.wrapping_add(process::USER_STACK_SIZE * PAGE_SIZE);
		let vdso = vdso::map(&mut mem_space, compat)?;
		// Initialize the userspace stack
		let aux = build_auxiliary(&self.0, load_base, &load_info, &vdso)?;
		let (_, init_stack_size) = get_init_stack_size(&self.0.argv, &self.0.envp, &aux, compat);
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
						compat,
					);
				});
			});
		}
		mem_space.set_brk_init(VirtAddr::from(load_info.load_end).align_to(PAGE_SIZE));
		Ok(ProgramImage {
			mem_space,
			compat,

			entry_point: load_info.entry_point,
			user_stack: VirtAddr::from(user_stack) - init_stack_size,
		})
	}
}
