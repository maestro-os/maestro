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
	cpu, elf,
	elf::{
		parser::ELFParser,
		relocation::{ELF32Rel, ELF32Rela, Relocation, GOT_SYM},
		ELF32ProgramHeader,
	},
	exec::vdso::MappedVDSO,
	file::{path::Path, perm::AccessProfile, vfs, File},
	memory,
	memory::vmem,
	process,
	process::{
		exec::{ExecInfo, Executor, ProgramImage},
		mem_space,
		mem_space::{residence::MapResidence, MapConstraint, MemSpace},
	},
};
use core::{
	alloc::AllocError,
	cmp::{max, min},
	ffi::c_void,
	mem::size_of,
	num::NonZeroUsize,
	ptr,
	ptr::null,
	slice,
};
use utils::{
	collections::{string::String, vec::Vec},
	errno,
	errno::EResult,
	io::IO,
	vec, TryClone,
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

/// Informations returned after loading an ELF program used to finish
/// initialization.
#[derive(Debug)]
struct ELFLoadInfo {
	/// The load base address
	load_base: *const c_void,
	/// The pointer to the end of loaded segments
	load_end: *const c_void,

	/// The pointer to the program header if present
	phdr: *const c_void,
	/// The length in bytes of an entry in the program headers table.
	phentsize: usize,
	/// The number of entries in the program headers table.
	phnum: usize,

	/// The pointer to the entry point
	entry_point: *const c_void,

	/// The load base of the interpreter program
	interp_load_base: Option<*const c_void>,
	/// The pointer to the entry point to be given to the interpreter
	interp_entry: Option<*const c_void>,
}

/// An entry of System V's Auxilary Vectors.
#[repr(C)]
struct AuxEntry {
	/// The entry's type.
	a_type: i32,
	/// The entry's value.
	a_val: isize,
}

/// Enumeration of possible values for an auxilary vector entry.
enum AuxEntryDescValue {
	/// A single number.
	Number(isize),
	/// A string of bytes.
	String(&'static [u8]),
}

/// Structure describing an auxilary vector entry.
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

/// Builds an auxilary vector.
///
/// Arguments:
/// - `exec_info` is the set of execution informations.
/// - `load_info` is the set of ELF load informations.
/// - `vdso` is the set of vDSO informations.
fn build_auxilary(
	exec_info: &ExecInfo,
	load_info: &ELFLoadInfo,
	vdso: &MappedVDSO,
) -> EResult<Vec<AuxEntryDesc>> {
	let mut aux = Vec::new();

	aux.push(AuxEntryDesc::new(
		AT_PHDR,
		AuxEntryDescValue::Number(load_info.phdr as _),
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
		AuxEntryDescValue::Number(memory::PAGE_SIZE as _),
	))?;

	if let Some(base) = load_info.interp_load_base {
		aux.push(AuxEntryDesc::new(
			AT_BASE,
			AuxEntryDescValue::Number(base as _),
		))?;
	}

	if let Some(entry) = load_info.interp_entry {
		aux.push(AuxEntryDesc::new(
			AT_ENTRY,
			AuxEntryDescValue::Number(entry as _),
		))?;
	}

	aux.push(AuxEntryDesc::new(AT_NOTELF, AuxEntryDescValue::Number(0)))?;
	aux.push(AuxEntryDesc::new(
		AT_UID,
		AuxEntryDescValue::Number(exec_info.path_resolution.access_profile.get_uid() as _),
	))?;
	aux.push(AuxEntryDesc::new(
		AT_EUID,
		AuxEntryDescValue::Number(exec_info.path_resolution.access_profile.get_euid() as _),
	))?;
	aux.push(AuxEntryDesc::new(
		AT_GID,
		AuxEntryDescValue::Number(exec_info.path_resolution.access_profile.get_gid() as _),
	))?;
	aux.push(AuxEntryDesc::new(
		AT_EGID,
		AuxEntryDescValue::Number(exec_info.path_resolution.access_profile.get_egid() as _),
	))?;
	aux.push(AuxEntryDesc::new(
		AT_PLATFORM,
		AuxEntryDescValue::String(crate::NAME.as_bytes()),
	))?;

	let hwcap = cpu::get_hwcap();
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
		AuxEntryDescValue::Number(vdso.ptr as _),
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
fn read_exec_file(file: &mut File, ap: &AccessProfile) -> EResult<Vec<u8>> {
	// Check that the file can be executed by the user
	if !ap.can_execute_file(file) {
		return Err(errno!(ENOEXEC));
	}

	let len = file.get_size().try_into().map_err(|_| AllocError)?;
	let mut image = vec![0u8; len]?;
	file.read(0, image.as_mut_slice())?;
	Ok(image)
}

/// The program executor for ELF files.
pub struct ELFExecutor<'s> {
	/// Execution information.
	info: ExecInfo<'s>,
}

impl<'s> ELFExecutor<'s> {
	/// Creates a new instance to execute the given program.
	///
	/// Arguments:
	/// - `uid` is the User ID of the executing user.
	/// - `gid` is the Group ID of the executing user.
	pub fn new(info: ExecInfo<'s>) -> EResult<Self> {
		Ok(Self {
			info,
		})
	}

	/// Returns two values:
	/// - The size in bytes of the buffer to store the arguments and environment variables, padding
	/// included.
	/// - The required size in bytes for the data to be written on the stack before the program
	/// starts.
	fn get_init_stack_size(
		argv: &[String],
		envp: &[String],
		aux: &[AuxEntryDesc],
	) -> (usize, usize) {
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

		// The size of the auxilary vector
		let aux_size = aux.len() * size_of::<AuxEntry>();
		// The size of the environment pointers + the null fourbyte
		let envp_size = envp.len() * 4 + 4;
		// The size of the argument pointers + the null fourbyte + argc
		let argv_size = argv.len() * 4 + 8;

		// The total size of the stack data in bytes
		let total_size = info_block_size + info_block_pad + aux_size + envp_size + argv_size;

		(info_block_size + info_block_pad, total_size)
	}

	// TODO Clean
	/// Initializes the stack data of the process according to the System V ABI.
	///
	/// Arguments:
	/// - `user_stack` the pointer to the user stack.
	/// - `argv` is the list of arguments.
	/// - `envp` is the environment.
	/// - `aux` is the auxilary vector.
	///
	/// The function returns the distance between the top of the stack and the
	/// new bottom after the data has been written.
	fn init_stack(
		&self,
		user_stack: *mut c_void,
		argv: &[String],
		envp: &[String],
		aux: &[AuxEntryDesc],
	) {
		let (info_size, total_size) = Self::get_init_stack_size(argv, envp, aux);

		// A slice on the stack representing the region which will containing the
		// arguments and environment variables
		let info_slice = unsafe {
			slice::from_raw_parts_mut((user_stack as usize - info_size) as *mut u8, info_size)
		};

		// A slice on the stack representing the region to fill
		let stack_slice = unsafe {
			slice::from_raw_parts_mut(
				(user_stack as usize - total_size) as *mut u32,
				total_size / size_of::<u32>(),
			)
		};

		// The offset in the information block
		let mut info_off = 0;
		// The offset in the pointers list
		let mut stack_off = 0;

		// Setting argc
		stack_slice[stack_off] = argv.len() as u32;
		stack_off += 1;

		// Setting arguments
		for arg in argv {
			// The offset of the beginning of the argument in the information block
			let begin = info_off;

			// Copying the argument into the information block
			for b in arg.iter() {
				info_slice[info_off] = *b;
				info_off += 1;
			}
			// Setting the nullbyte to end the string
			info_slice[info_off] = 0;
			info_off += 1;

			// Setting the argument's pointer
			stack_slice[stack_off] = &mut info_slice[begin] as *mut _ as u32;
			stack_off += 1;
		}
		// Setting the nullbyte to end argv
		stack_slice[stack_off] = 0;
		stack_off += 1;

		// Setting environment
		for var in envp {
			// The offset of the beginning of the variable in the information block
			let begin = info_off;

			// Copying the variable into the information block
			for b in var.iter() {
				info_slice[info_off] = *b;
				info_off += 1;
			}
			// Setting the nullbyte to end the string
			info_slice[info_off] = 0;
			info_off += 1;

			// Setting the variable's pointer
			stack_slice[stack_off] = &mut info_slice[begin] as *mut _ as u32;
			stack_off += 1;
		}
		// Setting the nullbytes to end envp
		stack_slice[stack_off] = 0;
		stack_off += 1;

		// Setting auxilary vector
		for a in aux {
			let val = match a.a_val {
				AuxEntryDescValue::Number(n) => n as _,

				AuxEntryDescValue::String(slice) => {
					// The offset of the beginning of the variable in the information block
					let begin = info_off;

					// Copying the string into the information block
					for b in slice {
						info_slice[info_off] = *b;
						info_off += 1;
					}
					// Setting the nullbyte to end the string
					info_slice[info_off] = 0;
					info_off += 1;

					&mut info_slice[begin] as *mut _ as _
				}
			};

			// Setting the entry
			stack_slice[stack_off] = a.a_type as _;
			stack_slice[stack_off + 1] = val;

			stack_off += 2;
		}
	}

	/// Allocates memory in userspace for an ELF segment.
	///
	/// If the segment isn't loadable, the function does nothing.
	///
	/// Arguments:
	/// - `load_base` is the address at which the executable is loaded.
	/// - `mem_space` is the memory space to allocate into.
	/// - `seg` is the segment for which the memory is allocated.
	///
	/// If loaded, the function return the pointer to the end of the segment in
	/// virtual memory.
	fn alloc_segment(
		load_base: *const c_void,
		mem_space: &mut MemSpace,
		seg: &ELF32ProgramHeader,
	) -> EResult<Option<*const c_void>> {
		// Loading only loadable segments
		if seg.p_type != elf::PT_LOAD && seg.p_type != elf::PT_PHDR {
			return Ok(None);
		}

		// Checking the alignment is correct
		if !seg.p_align.is_power_of_two() {
			return Err(errno!(EINVAL));
		}

		// The size of the padding before the segment
		let pad = seg.p_vaddr as usize % max(seg.p_align as usize, memory::PAGE_SIZE);
		// The pointer to the beginning of the segment in memory
		let mem_begin = unsafe { load_base.add(seg.p_vaddr as usize - pad) };
		// The length of the memory to allocate in pages
		let pages = (pad + seg.p_memsz as usize).div_ceil(memory::PAGE_SIZE);

		if let Some(pages) = NonZeroUsize::new(pages) {
			mem_space.map(
				MapConstraint::Fixed(mem_begin as _),
				pages,
				seg.get_mem_space_flags(),
				MapResidence::Normal,
			)?;
			// Pre-allocate the pages to make them writable
			mem_space.alloc(mem_begin, pages.get() * memory::PAGE_SIZE)?;
		}

		// The pointer to the end of the virtual memory chunk
		let mem_end = unsafe { mem_begin.add(pages * memory::PAGE_SIZE) };
		Ok(Some(mem_end as _))
	}

	/// Copies the segment's data into memory.
	///
	/// If the segment isn't loadable, the function does nothing.
	///
	/// Arguments:
	/// - `load_base` is the address at which the executable is loaded.
	/// - `seg` is the segment.
	/// - `image` is the ELF file image.
	fn copy_segment(load_base: *const c_void, seg: &ELF32ProgramHeader, image: &[u8]) {
		// Loading only loadable segments
		if seg.p_type != elf::PT_LOAD && seg.p_type != elf::PT_PHDR {
			return;
		}

		// A slice to the beginning of the segment's data in the file
		let file_begin = &image[seg.p_offset as usize];

		// The pointer to the beginning of the segment in the virtual memory
		let begin = unsafe { load_base.add(seg.p_vaddr as usize) as *mut _ };
		// The length of data to be copied from file
		let len = min(seg.p_memsz, seg.p_filesz) as usize;

		// Copying the segment's data
		unsafe {
			vmem::write_lock_wrap(|| ptr::copy_nonoverlapping(file_begin, begin, len));
		}
	}

	/// Loads the ELF file parsed by `elf` into the memory space `mem_space`.
	///
	/// Arguments:
	/// - `elf` is the ELF image.
	/// - `mem_space` is the memory space.
	/// - `load_base` is the base address at which the ELF is loaded.
	/// - `interp` tells whether the function loads an interpreter.
	fn load_elf(
		&self,
		elf: &ELFParser,
		mem_space: &mut MemSpace,
		load_base: *const c_void,
		interp: bool,
	) -> EResult<ELFLoadInfo> {
		// Allocating memory for segments
		let mut load_end = load_base;
		for seg in elf.iter_segments() {
			if let Some(end) = Self::alloc_segment(load_base, mem_space, seg)? {
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
			.filter(|seg| seg.p_type == elf::PT_PHDR)
			.map(|seg| seg.p_vaddr as *mut c_void)
			.next();
		let (phdr, phdr_needs_copy) = match phdr {
			Some(phdr) => (phdr, false),

			// Not phdr segment. Load it manually
			None => {
				let pages = phdr_size.div_ceil(memory::PAGE_SIZE);
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

		let mut entry_point = (load_base as usize + elf.hdr().e_entry as usize) as *const c_void;

		let mut interp_load_base = None;
		let mut interp_entry = None;

		// Loading the interpreter, if present
		let interp_path = elf.get_interpreter_path();
		if let Some(interp_path) = interp_path {
			// If the interpreter tries to load another interpreter, return an error
			if interp {
				return Err(errno!(EINVAL));
			}

			// Get file
			let interp_path = Path::new(interp_path)?;
			let interp_file_mutex =
				vfs::get_file_from_path(interp_path, self.info.path_resolution)?;
			let mut interp_file = interp_file_mutex.lock();

			let interp_image =
				read_exec_file(&mut interp_file, &self.info.path_resolution.access_profile)?;
			let interp_elf = ELFParser::new(interp_image.as_slice())?;
			let i_load_base = load_end as _; // TODO ASLR
			let load_info = self.load_elf(&interp_elf, mem_space, i_load_base, true)?;

			interp_load_base = Some(i_load_base as _);
			interp_entry =
				Some((load_base as usize + elf.hdr().e_entry as usize) as *const c_void);
			load_end = load_info.load_end;
			entry_point = load_info.entry_point;
		}

		// Switch to the process's vmem to write onto the virtual memory
		unsafe {
			vmem::switch(mem_space.get_vmem(), move || -> EResult<()> {
				// Copy segments' data
				for seg in elf.iter_segments() {
					Self::copy_segment(load_base, seg, elf.get_image());
				}

				// Copy phdr's data if necessary
				if phdr_needs_copy {
					let image_phdr = &elf.get_image()[(ehdr.e_phoff as usize)..];
					vmem::write_lock_wrap(|| {
						ptr::copy_nonoverlapping::<u8>(image_phdr.as_ptr(), phdr as _, phdr_size);
					});
				}

				// Perform relocations if no interpreter is present
				if !interp && interp_path.is_none() {
					// Closure returning a symbol
					let get_sym = |sym_section: u32, sym: u32| {
						let section = elf.get_section_by_index(sym_section as _)?;
						let sym = elf.get_symbol_by_index(section, sym as _)?;
						if sym.is_defined() {
							Some(load_base as u32 + sym.st_value)
						} else {
							None
						}
					};

					let got_sym = elf.get_symbol_by_name(GOT_SYM);
					for section in elf.iter_sections() {
						for rel in elf.iter_rel::<ELF32Rel>(section) {
							rel.perform(load_base as _, section, get_sym, got_sym)
								.map_err(|_| errno!(EINVAL))?;
						}
						for rela in elf.iter_rel::<ELF32Rela>(section) {
							rela.perform(load_base as _, section, get_sym, got_sym)
								.map_err(|_| errno!(EINVAL))?;
						}
					}
				}

				Ok(())
			})?;
		}

		Ok(ELFLoadInfo {
			load_base: load_base as _,
			load_end,

			phdr,
			phentsize,
			phnum,

			entry_point,

			interp_load_base,
			interp_entry,
		})
	}
}

impl<'s> Executor for ELFExecutor<'s> {
	// TODO Ensure there is no way to write in kernel space (check segments position
	// and relocations)
	// TODO Handle suid and sgid
	fn build_image(&self, file: &mut File) -> EResult<ProgramImage> {
		// The ELF file image
		let image = read_exec_file(file, &self.info.path_resolution.access_profile)?;
		// Parsing the ELF file
		let parser = ELFParser::new(image.as_slice())?;

		// The process's new memory space
		let mut mem_space = MemSpace::new()?;

		// Loading the ELF
		let load_info = self.load_elf(&parser, &mut mem_space, null::<c_void>(), false)?;

		// The user stack
		let user_stack = mem_space.map_stack(
			process::USER_STACK_SIZE.try_into().unwrap(),
			process::USER_STACK_FLAGS,
		)?;

		// Map the vDSO
		let vdso = vdso::map(&mut mem_space)?;

		// The auxiliary vector
		let aux = build_auxilary(&self.info, &load_info, &vdso)?;
		// The size in bytes of the initial data on the stack
		let init_stack_size = Self::get_init_stack_size(&self.info.argv, &self.info.envp, &aux).1;
		// Pre-allocate pages on the user stack to write the initial data
		{
			// The number of pages to allocate on the user stack to write the initial data
			let pages_count = init_stack_size.div_ceil(memory::PAGE_SIZE);
			// Check the data doesn't exceed the stack's size
			if pages_count >= process::USER_STACK_SIZE {
				return Err(errno!(ENOMEM));
			}
			// Allocate the pages on the stack to write the initial data
			let len = pages_count * memory::PAGE_SIZE;
			let begin = (user_stack as usize - len) as *const c_void;
			mem_space.alloc(begin, len)?;
		}

		// The initial pointer for `brk`
		let brk_ptr = unsafe { utils::align(load_info.load_end, memory::PAGE_SIZE) };
		mem_space.set_brk_init(brk_ptr as _);

		// Switch to the process's vmem to write onto the virtual memory
		unsafe {
			vmem::switch(mem_space.get_vmem(), move || {
				// Initializing the userspace stack
				self.init_stack(user_stack, &self.info.argv, &self.info.envp, &aux);
			});
		}

		let user_stack_begin = unsafe { user_stack.sub(init_stack_size) };

		Ok(ProgramImage {
			argv: self.info.argv.try_clone()?,

			mem_space,

			entry_point: load_info.entry_point,

			user_stack,
			user_stack_begin,
		})
	}
}
