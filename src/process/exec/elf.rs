//! This module implements ELF program execution with respects the System V ABI.

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
use crate::process::Process;
use crate::process::Regs;
use crate::process::exec::Executor;
use crate::process::mem_space::MemSpace;
use crate::process::signal::SignalHandler;
use crate::process;
use crate::util::IO;
use crate::util::container::vec::Vec;
use crate::util::math;

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
	/// Fills an auxilary vector for the given process `process`.
	fn fill_auxilary(process: &Process) -> Result<Vec<Self>, Errno> {
		let mut aux = Vec::new();
		aux.push(AuxEntry::new(AT_NULL, 0))?;
		aux.push(AuxEntry::new(AT_PAGESZ, memory::PAGE_SIZE as _))?;
		aux.push(AuxEntry::new(AT_UID, process.get_uid() as _))?;
		aux.push(AuxEntry::new(AT_EUID, process.get_euid() as _))?;
		aux.push(AuxEntry::new(AT_GID, process.get_gid() as _))?;
		aux.push(AuxEntry::new(AT_EGID, process.get_egid() as _))?;
		// TODO AT_SECURE

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
	let file_mutex = files_cache.as_mut().unwrap().get_file_from_path(&path)?;
	let mut file_lock = file_mutex.lock();
	let file = file_lock.get_mut();

	// Check that the file can be executed by the user
	if !file.can_execute(uid, gid) {
		return Err(errno::ENOEXEC);
	}

	// Allocating memory for the file's content
	let len = file.get_size();
	let mut image = unsafe {
		malloc::Alloc::new_zero(len as usize)?
	};

	// Reading the file
	file.read(0, image.get_slice_mut())?;

	Ok(image)
}

/// The program executor for ELF files.
pub struct ELFExecutor {
	/// The program image.
	image: malloc::Alloc<u8>,
}

impl ELFExecutor {
	/// Creates a new instance to execute the given program.
	/// `path` is the path to the program.
	/// `uid` is the User ID of the executing user.
	/// `gid` is the Group ID of the executing user.
	pub fn new(path: &Path, uid: Uid, gid: Gid) -> Result<Self, Errno> {
		Ok(Self {
			image: read_exec_file(path, uid, gid)?,
		})
	}

	/// Loads the interpreter from the given path for the given process.
	/// `mem_space` is the memory space on which the interpreter is loaded.
	/// `interp` is the path to the interpreter.
	/// If the memory space is not bound, the behaviour is undefined.
	/// `uid` is the User ID of the executing user.
	/// `gid` is the Group ID of the executing user.
	fn load_interpreter(&self, _mem_space: &mut MemSpace, uid: Uid, gid: Gid, interp: &Path)
		-> Result<(), Errno> {
		let _image = read_exec_file(interp, uid, gid)?;

		// TODO
		Ok(())
	}

	/// Returns two values:
	/// - The size in bytes of the buffer to store the arguments and environment variables, padding
	/// included.
	/// - The required size in bytes for the data to be written on the stack before the program
	/// starts.
	fn get_init_stack_size(argv: &[&str], envp: &[&str], aux: &Vec<AuxEntry>) -> (usize, usize) {
		// The size of the block storing the arguments and environment
		let mut info_block_size = 0;
		for e in envp {
			info_block_size += e.as_bytes().len() + 1;
		}
		for a in argv {
			info_block_size += a.as_bytes().len() + 1;
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
	/// The function returns the distance between the top of the stack and the new bottom after the
	/// data has been written.
	fn init_stack(&self, user_stack: *const c_void, argv: &[&str], envp: &[&str],
		aux: &Vec<AuxEntry>) {
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
			for b in arg.as_bytes() {
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
			for b in var.as_bytes() {
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

		for (i, a) in aux.iter().enumerate() {
			stack_slice[stack_off + i * 2] = a.a_type as _;
			stack_slice[stack_off + i * 2 + 1] = a.a_val as _;
		}
	}

	/// Allocates memory in userspace for an ELF segment.
	/// `load_base` is the address at which the executable is loaded.
	/// `mem_space` is the memory space to allocate into.
	/// `seg` is the segment for which the memory is allocated.
	fn alloc_segment(load_base: *const u8, mem_space: &mut MemSpace, seg: &ELF32ProgramHeader)
		-> Result<(), Errno> {
		if seg.p_type == elf::PT_LOAD {
			// The pointer to the beginning of the segment in the virtual memory
			let begin = unsafe {
				load_base.add(seg.p_vaddr as usize)
			};

			// The length of the segment in bytes
			let len = min(seg.p_memsz, seg.p_filesz) as usize;
			// The length of the segment in pages
			let pages = math::ceil_division(len, memory::PAGE_SIZE);

			// The mapping's flags
			let flags = seg.get_mem_space_flags();

			if pages > 0 {
				mem_space.map(Some(begin as _), pages, flags, None, 0)?;

				// Pre-allocating the pages to make them writable
				for i in 0..pages {
					mem_space.alloc((begin as usize + i * memory::PAGE_SIZE) as *const u8)?;
				}
			}
		}

		Ok(())
	}
}

impl Executor for ELFExecutor {
	// TODO Clean
	// TODO Ensure there is no way to write in kernel space (check segments position and
	// relocations)
	fn exec(&self, process: &mut Process, argv: &[&str], envp: &[&str]) -> Result<(), Errno> {
		debug_assert_eq!(process.state, crate::process::State::Running);

		// Parsing the ELF file
		let parser = ELFParser::new(self.image.get_slice())?;

		// The process's new memory space
		let mut mem_space = MemSpace::new()?;

		// Loading the interpreter, if any
		if let Some(interpreter_path) = parser.get_interpreter_path() {
			let interpreter_path = Path::from_str(interpreter_path, true)?;
			self.load_interpreter(&mut mem_space, process.get_euid(), process.get_egid(),
				&interpreter_path)?;
		}

		// FIXME Use 0x0 as a load base only if the program is non-relocatable
		// The base at which the program is loaded
		//let load_base = memory::PAGE_SIZE as *mut u8; // TODO Support ASLR
		let load_base = 0x0 as *mut u8; // TODO Support ASLR

		// Stores a result for the next iteration to allow the transmition of errors
		let mut result: Result<(), Errno> = Ok(());
		// Allocating memory for segments
		parser.foreach_segments(| seg | {
			result = Self::alloc_segment(load_base, &mut mem_space, seg);
			result.is_ok()
		});
		result?;

		// The kernel stack
		let kernel_stack = mem_space.map_stack(None, process::KERNEL_STACK_SIZE,
			process::KERNEL_STACK_FLAGS)?;
		// The user stack
		let user_stack = mem_space.map_stack(None, process::USER_STACK_SIZE,
			process::USER_STACK_FLAGS)?;

		// The auxilary vector
		let aux = AuxEntry::fill_auxilary(process)?;

		// The size in bytes of the initial data on the stack
		let total_size = Self::get_init_stack_size(argv, envp, &aux).1;
		// Pre-allocating pages on the user stack to write the initial data
		{
			// The number of pages to allocate on the user stack to write the initial data
			let pages_count = math::ceil_division(total_size, memory::PAGE_SIZE);
			// Checking that the data doesn't exceed the stack's size
			if pages_count >= process::USER_STACK_SIZE {
				return Err(errno::ENOMEM);
			}

			// Allocating the pages on the stack to write the initial data
			for i in 0..pages_count {
				let ptr = (user_stack as usize - (i + 1) * memory::PAGE_SIZE) as *const c_void;
				mem_space.alloc(ptr)?;
			}
		}

		// Switching to the process's vmem to write onto the virtual memory
		vmem::switch(mem_space.get_vmem().as_ref(), || {
			// Copying the segments' content into the virtual memory
			parser.foreach_segments(| seg | {
				if seg.p_type == elf::PT_LOAD {
					// The pointer to the beginning of the segment in the file
					let file_begin = &self.image[seg.p_offset as usize];
					// The pointer to the beginning of the segment in the virtual memory
					let begin = unsafe {
						load_base.add(seg.p_vaddr as usize)
					};
					// The length of the segment in bytes
					let len = min(seg.p_memsz, seg.p_filesz) as usize;

					unsafe { // Safe because the module ELF image is valid at this point
						vmem::write_lock_wrap(|| {
							ptr::copy_nonoverlapping::<u8>(file_begin, begin, len);
						});
					}
				}

				true
			});

			// Closure returning a symbol from its name
			let get_sym = | name: &str | parser.get_symbol_by_name(name);

			// Closure returning the value for a given symbol
			let get_sym_val = | sym_section: u32, sym: u32 | {
				let section = parser.get_section_by_index(sym_section)?;
				let sym = parser.get_symbol_by_index(section, sym)?;

				if sym.is_defined() {
					Some(load_base as u32 + sym.st_value)
				} else {
					None // TODO Prepare for the interpreter?
				}
			};

			parser.foreach_rel(| section, rel | {
				unsafe {
					rel.perform(load_base as _, section, get_sym, get_sym_val);
				}
				true
			});
			parser.foreach_rela(| section, rela | {
				unsafe {
					rela.perform(load_base as _, section, get_sym, get_sym_val);
				}
				true
			});

			// Initializing the userspace stack
			self.init_stack(user_stack, argv, envp, &aux);
		});

		// Setting the new memory space to the process
		process.set_mem_space(Some(mem_space));

		// Setting the process's stacks
		process.user_stack = Some(user_stack);
		process.kernel_stack = Some(kernel_stack);

		// Resetting signals
		process.signals_bitfield.clear_all();
		for i in 0..process.signal_handlers.len() {
			process.signal_handlers[i] = SignalHandler::Default;
		}

		// Setting the process's entry point
		let hdr = parser.get_header();

		// Setting the process's registers
		let mut regs = Regs::default();
		regs.esp = (user_stack as usize - total_size) as _;
		regs.eip = load_base as u32 + hdr.e_entry;
		process.regs = regs;

		Ok(())
	}
}
