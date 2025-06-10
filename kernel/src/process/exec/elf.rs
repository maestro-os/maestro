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
	elf::{
		ET_DYN, ET_EXEC, PT_LOAD,
		parser::{Class, ELFParser, ProgramHeader},
	},
	file::{File, FileType, O_RDONLY, vfs},
	memory::{COMPAT_PROCESS_END, PROCESS_END, VirtAddr, vmem},
	process::{
		USER_STACK_SIZE,
		exec::{ExecInfo, ProgramImage, vdso::MappedVDSO},
		mem_space,
		mem_space::{MAP_ANONYMOUS, MAP_FIXED, MAP_PRIVATE, MemSpace, PROT_READ, PROT_WRITE},
	},
};
use core::{cmp::max, hint::unlikely, num::NonZeroUsize, ops::Add, ptr, slice};
use utils::{
	collections::{path::Path, vec::Vec},
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
	load_end: VirtAddr,

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
enum AuxEntryDescValue<'s> {
	/// A single number.
	Number(usize),
	/// A string of bytes.
	String(&'s [u8]),
}

/// An auxiliary vector entry.
struct AuxEntryDesc<'s> {
	/// The entry's type.
	pub a_type: i32,
	/// The entry's value.
	pub a_val: AuxEntryDescValue<'s>,
}

/// Builds an auxiliary vector.
///
/// Arguments:
/// - `exec_path` the executable file path
/// - `exec_info` is the set of execution information
/// - `interp_load_base` is the base address at which the interpreter is loaded
/// - `load_info` is the set of ELF load information
/// - `vdso` is the set of vDSO information
fn build_auxiliary<'s>(
	exec_path: &'s Path,
	exec_info: &ExecInfo,
	interp_load_base: VirtAddr,
	load_info: &ELFLoadInfo,
	vdso: &MappedVDSO,
) -> AllocResult<Vec<AuxEntryDesc<'s>>> {
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
			a_val: AuxEntryDescValue::Number(interp_load_base.0),
		},
		AuxEntryDesc {
			a_type: AT_ENTRY,
			a_val: AuxEntryDescValue::Number(load_info.entry_point.0),
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
			a_val: AuxEntryDescValue::String(exec_path.as_bytes()),
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

/// Maps the segment `seg` in memory.
///
/// If the segment is not loadable, the function does nothing.
///
/// Arguments:
/// - `file` is the file from which the segment is mapped
/// - `mem_space` is the memory space to allocate into
/// - `load_base` is the base address at which the executable is loaded
/// - `seg` is the segment for which the memory is allocated
///
/// If loaded, the function return the pointer to the end of the segment in
/// virtual memory.
fn map_segment(
	file: Arc<File>,
	mem_space: &MemSpace,
	load_base: VirtAddr,
	seg: &ProgramHeader,
) -> EResult<VirtAddr> {
	if unlikely(seg.p_memsz < seg.p_filesz) {
		return Err(errno!(ENOEXEC));
	}
	if unlikely(seg.p_align as usize != PAGE_SIZE) {
		return Err(errno!(ENOEXEC));
	}
	let page_start = seg.p_vaddr as usize & !(PAGE_SIZE - 1);
	let page_off = seg.p_vaddr as usize & (PAGE_SIZE - 1);
	let addr = load_base + page_start;
	let size = seg.p_memsz as usize + page_off;
	let pages = size.div_ceil(PAGE_SIZE);
	if let Some(pages) = NonZeroUsize::new(pages) {
		mem_space.map(
			addr,
			pages,
			seg.mmap_prot(),
			MAP_PRIVATE | MAP_FIXED,
			Some(file),
			seg.p_offset - page_off as u64,
		)?;
	}
	// The pointer to the end of the virtual memory chunk
	let mem_end = addr.add(pages * PAGE_SIZE);
	Ok(mem_end)
}

/// Loads the ELF file parsed by `elf` into the memory space `mem_space`.
///
/// Arguments:
/// - `file` is the file containing the ELF image
/// - `elf` is the ELF image
/// - `mem_space` is the memory space
/// - `load_base` is the base address at which the ELF is loaded
fn load_elf(
	file: &Arc<File>,
	elf: &ELFParser,
	mem_space: &Arc<MemSpace>,
	load_base: VirtAddr,
) -> EResult<ELFLoadInfo> {
	let ehdr = elf.hdr();
	let mut load_end = load_base;
	let mut phdr_addr = VirtAddr(0);
	unsafe {
		MemSpace::switch(mem_space, |mem_space| -> EResult<()> {
			// Map segments
			for seg in elf.iter_segments() {
				if seg.p_type != PT_LOAD {
					continue;
				}
				let seg_end = map_segment(file.clone(), mem_space, load_base, &seg)?;
				load_end = max(seg_end, load_end);
				// If the segment contains the phdr, keep its address
				if (seg.p_offset..seg.p_offset + seg.p_filesz).contains(&ehdr.e_phoff) {
					phdr_addr = load_base + (ehdr.e_phoff - seg.p_offset + seg.p_vaddr) as usize;
				}
			}
			// Zero the end of segments when needed
			vmem::write_ro(|| {
				vmem::smap_disable(|| {
					for seg in elf.iter_segments() {
						if seg.p_type != PT_LOAD {
							continue;
						}
						if seg.p_memsz <= seg.p_filesz {
							continue;
						}
						let begin = load_base.add(seg.p_vaddr as usize + seg.p_filesz as usize);
						let end = load_base
							.add(seg.p_vaddr as usize + seg.p_memsz as usize)
							.next_multiple_of(PAGE_SIZE);
						let len = end - begin.0;
						let slice = slice::from_raw_parts_mut(begin.as_ptr::<u8>(), len);
						slice.fill(0);
					}
				});
			});
			Ok(())
		})?;
	}
	Ok(ELFLoadInfo {
		load_end,

		phdr: phdr_addr,
		phentsize: ehdr.e_phentsize as _,
		phnum: ehdr.e_phnum as _,

		entry_point: load_base + elf.hdr().e_entry as usize,
	})
}

/// Computes the size of the initial data on the stack.
///
/// `compat` indicates whether userspace runs in compatibility mode.
///
/// Returns the size of the "information" part, and the total size on the stack (including the
/// "information" part).
fn get_init_stack_size(info: &ExecInfo, aux: &[AuxEntryDesc], compat: bool) -> (usize, usize) {
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
		.chain(info.envp.iter().map(|e| e.len() + 1))
		.chain(info.argv.iter().map(|a| a.len() + 1))
		.sum::<usize>()
		// Add padding before the information block allowing to preserve stack alignment
		.next_multiple_of(size);
	// The size of the auxiliary vector
	let aux_size = aux.len() * (size * 2);
	// The size of the environment pointers + null
	let envp_size = (info.envp.len() + 1) * size;
	// The size of the argument pointers + null + argc
	let argv_size = (info.argv.len() + 2) * size;
	// The total size of the stack data in bytes
	let total_size = info_block_size + aux_size + envp_size + argv_size;
	(info_block_size, total_size)
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
	info: &ExecInfo,
	aux: &[AuxEntryDesc],
	exe_info: &mut mem_space::ExeInfo,
	compat: bool,
) {
	let (info_size, total_size) = get_init_stack_size(info, aux, compat);
	let mut info_ptr = user_stack.sub(info_size);
	let mut args_ptr = user_stack.sub(total_size);
	// Push argc
	write_val(&mut args_ptr, info.argv.len(), compat);
	// Set argv
	exe_info.argv_begin = VirtAddr::from(info_ptr);
	for arg in &info.argv {
		write_val(&mut args_ptr, info_ptr as _, compat);
		copy_string(&mut info_ptr, arg);
	}
	// Set the nul byte to end argv
	write_val(&mut args_ptr, 0, compat);
	exe_info.argv_end = VirtAddr::from(info_ptr);
	// Set environment
	exe_info.envp_begin = exe_info.argv_end;
	for var in &info.envp {
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

// TODO Handle suid and sgid
/// Builds a program image from the given executable file.
///
/// Arguments:
/// - `ent` is the program's file
/// - `info` is the set execution information for the program
#[inline]
pub fn exec(ent: Arc<vfs::Entry>, info: ExecInfo) -> EResult<ProgramImage> {
	// Check the file can be executed by the user
	let stat = ent.stat();
	if unlikely(stat.get_type() != Some(FileType::Regular)) {
		return Err(errno!(EACCES));
	}
	if unlikely(!info.path_resolution.access_profile.can_execute_file(&stat)) {
		return Err(errno!(EACCES));
	}
	// Read and parse file
	let file = File::open_entry(ent.clone(), O_RDONLY)?;
	let image = file.read_all()?;
	let parser = ELFParser::new(&image)?;
	if unlikely(!matches!(parser.hdr().e_type, ET_EXEC | ET_DYN)) {
		return Err(errno!(ENOEXEC));
	}
	// Determine load base
	let mut load_base = VirtAddr(0);
	if parser.hdr().e_type == ET_DYN {
		// TODO ASLR
		load_base = VirtAddr(PAGE_SIZE);
	}
	// Initialize memory space
	let load_end = parser.get_load_size();
	let compat = parser.class() == Class::Bit32;
	let mut mem_space = MemSpace::new(ent, load_end, compat)?;
	// Load program
	let load_info = load_elf(&file, &parser, &mem_space, load_base)?;
	let mut entry_point = load_info.entry_point;
	// Compute the user stack address
	let user_stack_addr = if !compat {
		PROCESS_END - (USER_STACK_SIZE + 1) * PAGE_SIZE
	} else {
		COMPAT_PROCESS_END - (USER_STACK_SIZE + 1) * PAGE_SIZE
	};
	// If using an interpreter, load it
	let mut interp_load_base = VirtAddr(0);
	if let Some(interp) = parser.get_interpreter_path() {
		let interp = Path::new(interp)?;
		let interp_ent = vfs::get_file_from_path(interp, info.path_resolution)?;
		// Check the file can be executed by the user
		let stat = interp_ent.stat();
		if unlikely(stat.get_type() != Some(FileType::Regular)) {
			return Err(errno!(EACCES));
		}
		if unlikely(!info.path_resolution.access_profile.can_execute_file(&stat)) {
			return Err(errno!(EACCES));
		}
		// Read and parse file
		let file = File::open_entry(interp_ent, O_RDONLY)?;
		let image = file.read_all()?;
		let parser = ELFParser::new(&image)?;
		// Cannot load the interpreter at the beginning since it might be used by the program
		// itself
		if unlikely(parser.hdr().e_type != ET_DYN) {
			return Err(errno!(ENOEXEC));
		}
		// Subtract one page to leave a space in between the stack and the interpreter
		interp_load_base = user_stack_addr - PAGE_SIZE - parser.get_load_size().0; // TODO ASLR
		let load_info = load_elf(&file, &parser, &mem_space, interp_load_base)?;
		entry_point = load_info.entry_point;
	}
	// Allocate the userspace stack. We add one page to account for the copy buffer
	let user_stack = mem_space
		.map(
			user_stack_addr,
			USER_STACK_SIZE.try_into().unwrap(),
			PROT_READ | PROT_WRITE, // TODO PT_GNU_STACK
			MAP_PRIVATE | MAP_ANONYMOUS,
			None,
			0,
		)?
		.add(USER_STACK_SIZE * PAGE_SIZE);
	// Map vDSO
	let vdso = vdso::map(&mem_space, compat)?;
	// Initialize the userspace stack
	let exec_path = vfs::Entry::get_path(&mem_space.exe_info.exe)?;
	let aux = build_auxiliary(&exec_path, &info, interp_load_base, &load_info, &vdso)?;
	let (_, init_stack_size) = get_init_stack_size(&info, &aux, compat);
	let mut exe_info = mem_space.exe_info.clone();
	unsafe {
		MemSpace::switch(&mem_space, |_| {
			vmem::smap_disable(|| -> EResult<()> {
				init_stack(user_stack.as_ptr(), &info, &aux, &mut exe_info, compat);
				Ok(())
			})
		})?;
	}
	// Set immutable fields
	let m = Arc::as_mut(&mut mem_space).unwrap(); // Cannot fail since no one else hold a reference
	m.exe_info = exe_info;
	Ok(ProgramImage {
		mem_space,
		compat,

		entry_point,
		user_stack: user_stack - init_stack_size,
	})
}
