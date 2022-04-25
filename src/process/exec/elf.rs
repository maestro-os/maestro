//! This module implements ELF program execution with respects the System V ABI.

use core::cmp::max;
use core::cmp::min;
use core::ffi::c_void;
use core::mem::size_of;
use core::ptr;
use core::slice;
use core::str;
use crate::elf::ELF32ProgramHeader;
use crate::elf::parser::ELFParser;
use crate::elf::relocation::Relocation;
use crate::elf;
use crate::errno::Errno;
use crate::errno;
use crate::file::Gid;
use crate::file::Uid;
use crate::file::fcache;
use crate::file::path::Path;
use crate::memory::malloc;
use crate::memory::vmem;
use crate::memory;
use crate::process::exec::ExecInfo;
use crate::process::exec::Executor;
use crate::process::exec::ProgramImage;
use crate::process::mem_space::MemSpace;
use crate::process;
use crate::util::IO;
use crate::util::container::vec::Vec;
use crate::util::math;
use crate::util;

/// Used to define the end of the entries list.
const AT_NULL: i32 = 0;
/// Entry with no meaning, to be ignored.
const AT_IGNORE: i32 = 1;
/// Entry containing a file descriptor to the application object file in case the program is run
/// using an interpreter.
const AT_EXECFD: i32 = 2;
/// Entry containing a pointer to the program header table for the interpreter.
const AT_PHDR: i32 = 3;
/// The size in bytes of one entry in the program header table to which AT_PHDR points.
const AT_PHENT: i32 = 4;
/// The number of entries in the program header table to which AT_PHDR points.
const AT_PHNUM: i32 = 5;
/// The system's page size in bytes.
const AT_PAGESZ: i32 = 6;
/// The base address at which the interpreter program was loaded in memory.
const AT_BASE: i32 = 7;
/// Contains flags.
const AT_FLAGS: i32 = 8;
/// Entry with the pointer to the entry point of the program to which the interpreter should
/// transfer control.
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

/// An entry of System V's Auxilary Vectors.
#[repr(C)]
struct AuxEntry {
	/// The entry's type.
	a_type: i32,
	/// The entry's value.
	a_val: isize,
}

impl AuxEntry {
	/// Creates a new instance with the given type `a_type` and value `a_val`.
	pub fn new(a_type: i32, a_val: isize) -> Self {
		Self {
			a_type,
			a_val,
		}
	}

	// TODO Add interpreter support
	/// Fills an auxilary vector with execution informations `info`.
	/// `phdr` is the pointer to the program headers array loaded in the process's memory.
	/// `parser` is a reference to the ELF parser.
	fn fill_auxilary(info: &ExecInfo, phdr: Option<*const c_void>,
		parser: &ELFParser) -> Result<Vec<Self>, Errno> {
		let mut aux = Vec::new();

		if let Some(phdr) = phdr {
			aux.push(AuxEntry::new(AT_PHDR, phdr as _))?;
			aux.push(AuxEntry::new(AT_PHENT, parser.get_header().get_phentsize() as _))?;
			aux.push(AuxEntry::new(AT_PHNUM, parser.get_header().get_phnum() as _))?;
		}

		aux.push(AuxEntry::new(AT_PAGESZ, memory::PAGE_SIZE as _))?;
		aux.push(AuxEntry::new(AT_NOTELF, 0))?;
		aux.push(AuxEntry::new(AT_UID, info.uid as _))?;
		aux.push(AuxEntry::new(AT_EUID, info.euid as _))?;
		aux.push(AuxEntry::new(AT_GID, info.gid as _))?;
		aux.push(AuxEntry::new(AT_EGID, info.egid as _))?;
		// TODO AT_SECURE
		aux.push(AuxEntry::new(AT_NULL, 0))?;

		Ok(aux)
	}
}

/// Reads the file at the given path `path`. If the file is not executable, the function returns an
/// error.
/// `uid` is the User ID of the executing user.
/// `gid` is the Group ID of the executing user.
fn read_exec_file(path: &Path, uid: Uid, gid: Gid) -> Result<malloc::Alloc<u8>, Errno> {
	let mutex = fcache::get();
	let mut guard = mutex.lock();
	let files_cache = guard.get_mut();

	// Getting the file from path
	let file_mutex = files_cache.as_mut().unwrap().get_file_from_path(path, uid, gid, true)?;
	let mut file_lock = file_mutex.lock();
	let file = file_lock.get_mut();

	// Check that the file can be executed by the user
	if !file.can_execute(uid, gid) {
		return Err(errno!(ENOEXEC));
	}

	// Allocating memory for the file's content
	let len = file.get_size();
	let mut image = unsafe {
		malloc::Alloc::new_zero(len as usize)?
	};

	// Reading the file
	file.read(0, image.as_slice_mut())?;

	Ok(image)
}

/// The program executor for ELF files.
pub struct ELFExecutor<'a> {
	/// The program image.
	image: malloc::Alloc<u8>,
	/// Execution informations.
	info: ExecInfo<'a>,
}

impl<'a> ELFExecutor<'a> {
	/// Creates a new instance to execute the given program.
	/// `path` is the path to the program.
	/// `uid` is the User ID of the executing user.
	/// `gid` is the Group ID of the executing user.
	pub fn new(path: &Path, info: ExecInfo<'a>) -> Result<Self, Errno> {
		Ok(Self {
			image: read_exec_file(path, info.euid, info.egid)?,
			info,
		})
	}

	/// Returns two values:
	/// - The size in bytes of the buffer to store the arguments and environment variables, padding
	/// included.
	/// - The required size in bytes for the data to be written on the stack before the program
	/// starts.
	fn get_init_stack_size(argv: &[&[u8]], envp: &[&[u8]], aux: &[AuxEntry])
		-> (usize, usize) {
		// The size of the block storing the arguments and environment
		let mut info_block_size = 0;
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
	/// `user_stack` the pointer to the user stack.
	/// `argv` is the list of arguments.
	/// `envp` is the environment.
	/// `aux` is the auxilary vector.
	/// The function returns the distance between the top of the stack and the new bottom after the
	/// data has been written.
	fn init_stack(&self, user_stack: *const c_void, argv: &[&[u8]], envp: &[&[u8]],
		aux: &[AuxEntry]) {
		let (info_size, total_size) = Self::get_init_stack_size(argv, envp, aux);

		// A slice on the stack representing the region which will containing the arguments and
		// environment variables
		let info_slice = unsafe {
			slice::from_raw_parts_mut((user_stack as usize - info_size) as *mut u8, info_size)
		};

		// A slice on the stack representing the region to fill
		let stack_slice = unsafe {
			slice::from_raw_parts_mut((user_stack as usize - total_size) as *mut u32,
				total_size / size_of::<u32>())
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

		// Setting the auxilary vector
		for a in aux.iter() {
			stack_slice[stack_off] = a.a_type as _;
			stack_slice[stack_off + 1] = a.a_val as _;

			stack_off += 2;
		}
	}

	/// Allocates memory in userspace for an ELF segment.
	/// If the segment isn't loadable, the function does nothing.
	/// `load_base` is the address at which the executable is loaded.
	/// `mem_space` is the memory space to allocate into.
	/// `seg` is the segment for which the memory is allocated.
	/// If loaded, the function return the pointer to the end of the segment in virtual memory.
	fn alloc_segment(load_base: *const u8, mem_space: &mut MemSpace, seg: &ELF32ProgramHeader)
		-> Result<Option<*const c_void>, Errno> {
		// Loading only loadable segments
		if seg.p_type != elf::PT_LOAD {
			return Ok(None);
		}

		// Checking the alignment is correct
		if !math::is_power_of_two(seg.p_align) {
			return Err(errno!(EINVAL));
		}

		// The size of the padding before the segment
		let pad = seg.p_vaddr as usize % max(seg.p_align as usize, memory::PAGE_SIZE);
		// The pointer to the beginning of the segment in memory
		let mem_begin = unsafe {
			load_base.add(seg.p_vaddr as usize - pad)
		};
		// The length of the memory to allocate in pages
		let pages = math::ceil_division(pad + seg.p_memsz as usize, memory::PAGE_SIZE);

		if pages > 0 {
			mem_space.map(Some(mem_begin as _), pages, seg.get_mem_space_flags(), None, 0)?;

			// TODO Lazy allocation
			// Pre-allocating the pages to make them writable
			for i in 0..pages {
				mem_space.alloc((mem_begin as usize + i * memory::PAGE_SIZE) as *const u8)?;
			}
		}

		// The pointer to the end of the virtual memory chunk
		let mem_end = unsafe {
			mem_begin.add(pages * memory::PAGE_SIZE)
		};
		Ok(Some(mem_end as _))
	}

	/// Copies the segment's data into memory.
	/// If the segment isn't loadable, the function does nothing.
	/// `load_base` is the address at which the executable is loaded.
	/// `seg` is the segment.
	/// `image` is the ELF file image.
	fn copy_segment(load_base: *const u8, seg: &ELF32ProgramHeader, image: &[u8]) {
		// Loading only loadable segments
		if seg.p_type != elf::PT_LOAD {
			return;
		}

		// The pointer to the beginning of the segment in the virtual memory
		let begin = unsafe {
			load_base.add(seg.p_vaddr as usize) as *mut _
		};
		// The length of the segment in bytes
		let len = min(seg.p_memsz, seg.p_filesz) as usize;
		// A slice to the beginning of the segment's data in the file
		let file_begin = &image[seg.p_offset as usize];

		// Copying the segment's data
		unsafe { // Safe because the module ELF image is valid at this point
			vmem::write_lock_wrap(|| {
				ptr::copy_nonoverlapping::<u8>(file_begin, begin, len);
			});
		}
	}

	/// Loads the ELF file parsed by `elf` into the memory space `mem_space` at the base address
	/// `load_base`.
	/// `interp` tells whether the function loads an interpreter.
	/// The function returns:
	/// - The pointer to the end of loaded segments
	/// - The pointer to the program header if present.
	/// - The pointer to the entry point.
	fn load_elf(&self, elf: &ELFParser, mem_space: &mut MemSpace, load_base: *mut u8,
		interp: bool) -> Result<(*const c_void, Option<*const c_void>, *const c_void), Errno> {
		// Allocating memory for segments
		let mut load_end: Result<*const c_void, Errno> = Ok(load_base as _);
		// The pointer to the program header table in memory
		let mut phdr: Option<*const c_void> = None;
		elf.foreach_segments(| seg | {
			load_end = Self::alloc_segment(load_base, mem_space, seg).map(| end | {
				if let Some(end) = end {
					max(end, load_end.unwrap())
				} else {
					load_end.unwrap()
				}
			});

			// If PHDR, keep the pointer
			if seg.p_type == elf::PT_PHDR {
				phdr = Some(seg.p_vaddr as _);
			}

			load_end.is_ok()
		});
		let mut load_end = load_end?;
		let mut entry_point = (load_base as usize + elf.get_header().e_entry as usize)
			as *const c_void;

		// Loading the interpreter, if present
		if let Some(interp_path) = elf.get_interpreter_path() {
			// If the interpreter tries to load another interpreter, return an error
			if interp {
				return Err(errno!(EINVAL));
			}

			let interp_path = Path::from_str(interp_path, true)?;
			let interp_image = read_exec_file(&interp_path, self.info.euid, self.info.egid)?;
			let interp_elf = ELFParser::new(interp_image.as_slice())?;
			(load_end, _, entry_point)
				= self.load_elf(&interp_elf, mem_space, load_end as _, true)?;
		}

		// Switching to the process's vmem to write onto the virtual memory
		vmem::switch(mem_space.get_vmem().as_ref(), || {
			// Copying segments' data
			elf.foreach_segments(| seg | {
				Self::copy_segment(load_base, seg, self.image.as_slice());
				true
			});

			// Closure returning a symbol from its name
			let get_sym = | name: &str | elf.get_symbol_by_name(name);

			// Closure returning the value for a given symbol
			let get_sym_val = | sym_section: u32, sym: u32 | {
				let section = elf.get_section_by_index(sym_section)?;
				let sym = elf.get_symbol_by_index(section, sym)?;

				if sym.is_defined() {
					Some(load_base as u32 + sym.st_value)
				} else {
					None // TODO Prepare for the interpreter?
				}
			};

			// Performing relocations
			elf.foreach_rel(| section, rel | {
				unsafe {
					rel.perform(load_base as _, section, get_sym, get_sym_val);
				}
				true
			});
			elf.foreach_rela(| section, rela | {
				unsafe {
					rela.perform(load_base as _, section, get_sym, get_sym_val);
				}
				true
			});
		});

		Ok((load_end, phdr, entry_point))
	}
}

impl<'a> Executor<'a> for ELFExecutor<'a> {
	// TODO Ensure there is no way to write in kernel space (check segments position and
	// relocations)
	// TODO Handle suid and sgid
	fn build_image(&'a self) -> Result<ProgramImage, Errno> {
		// Parsing the ELF file
		let parser = ELFParser::new(self.image.as_slice())?;

		// The process's new memory space
		let mut mem_space = MemSpace::new()?;

		// FIXME Use 0x0 as a load base only if the program is non-relocatable
		// The base at which the program is loaded
		let load_base = 0x0 as *mut u8; // TODO Support ASLR

		// Loading the ELF
		let (load_end, phdr, entry_point)
			= self.load_elf(&parser, &mut mem_space, load_base, false)?;

		// The user stack
		let user_stack = mem_space.map_stack(None, process::USER_STACK_SIZE,
			process::USER_STACK_FLAGS)?;

		// The auxilary vector
		let aux = AuxEntry::fill_auxilary(&self.info, phdr, &parser)?;

		// The size in bytes of the initial data on the stack
		let total_size = Self::get_init_stack_size(self.info.argv, self.info.envp, &aux).1;
		// Pre-allocating pages on the user stack to write the initial data
		{
			// The number of pages to allocate on the user stack to write the initial data
			let pages_count = math::ceil_division(total_size, memory::PAGE_SIZE);
			// Checking that the data doesn't exceed the stack's size
			if pages_count >= process::USER_STACK_SIZE {
				return Err(errno!(ENOMEM));
			}

			// Allocating the pages on the stack to write the initial data
			for i in 0..pages_count {
				let ptr = (user_stack as usize - (i + 1) * memory::PAGE_SIZE) as *const c_void;
				mem_space.alloc(ptr)?;
			}
		}

		// The initial pointer for `brk`
		let brk_ptr = util::align(load_end, memory::PAGE_SIZE);
		mem_space.set_brk_init(brk_ptr);

		// Switching to the process's vmem to write onto the virtual memory
		vmem::switch(mem_space.get_vmem().as_ref(), || {
			// Initializing the userspace stack
			self.init_stack(user_stack, self.info.argv, self.info.envp, &aux);
		});

		// The kernel stack
		let kernel_stack = mem_space.map_stack(None, process::KERNEL_STACK_SIZE,
			process::KERNEL_STACK_FLAGS)?;

		Ok(ProgramImage {
			mem_space,

			entry_point,

			user_stack,
			user_stack_begin: (user_stack as usize - total_size) as _,

			kernel_stack,
		})
	}
}
