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
use crate::file::path::Path;
use crate::file;
use crate::memory::malloc;
use crate::memory::vmem;
use crate::memory;
use crate::process::Process;
use crate::process::Regs;
use crate::process::exec::Executor;
use crate::process::mem_space::MemSpace;
use crate::process::mem_space::{MAPPING_FLAG_WRITE, MAPPING_FLAG_USER, MAPPING_FLAG_NOLAZY};
use crate::process::signal::SignalHandler;
use crate::process::signal;
use crate::process;
use crate::util::math;

/// The size of the userspace stack of a process in number of pages.
const USER_STACK_SIZE: usize = 2048;
/// The flags for the userspace stack mapping.
const USER_STACK_FLAGS: u8 = MAPPING_FLAG_WRITE | MAPPING_FLAG_USER;
/// The size of the kernelspace stack of a process in number of pages.
const KERNEL_STACK_SIZE: usize = 64;
/// The flags for the kernelspace stack mapping.
const KERNEL_STACK_FLAGS: u8 = MAPPING_FLAG_WRITE | MAPPING_FLAG_NOLAZY;

/// Reads the file at the given path `path`. If the file is not executable, the function returns an
/// error.
/// `uid` is the User ID of the executing user.
/// `gid` is the Group ID of the executing user.
fn read_exec_file(path: &Path, uid: Uid, gid: Gid) -> Result<malloc::Alloc<u8>, Errno> {
	let mutex = file::get_files_cache();
	let mut guard = mutex.lock(true);
	let files_cache = guard.get_mut();

	// Getting the file from path
	let mut file_mutex = files_cache.get_file_from_path(&path)?;
	let file_lock = file_mutex.lock(true);
	let file = file_lock.get();

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
	fn get_init_stack_size(argv: &[&str], envp: &[&str]) -> (usize, usize) {
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

		// The size of the environment pointers + the null fourbyte
		let envp_size = envp.len() * 4 + 4;
		// The size of the argument pointers + the null fourbyte + argc
		let argv_size = argv.len() * 4 + 8;
		// The total size of the stack data in bytes
		let total_size = info_block_size + info_block_pad + 4 + envp_size + argv_size;

		(info_block_size + info_block_pad, total_size)
	}

	// TODO Clean
	/// Initializes the stack data of the process according to the System V ABI.
	/// `user_stack` the pointer to the user stack.
	/// `argv` is the list of arguments.
	/// `envp` is the environment.
	/// The function returns the distance between the top of the stack and the new bottom after the
	/// data has been written.
	fn init_stack(&self, user_stack: *const c_void, argv: &[&str], envp: &[&str]) {
		let (info_size, total_size) = Self::get_init_stack_size(argv, envp);

		// A slice on the stack representing the region which will containing the arguments and
		// environment variables
		let info_slice = unsafe {
			slice::from_raw_parts_mut((user_stack as usize - info_size) as *mut u8,
				info_size / size_of::<u8>())
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
		for i in 0..2 {
			stack_slice[stack_off + i] = 0;
		}
		stack_off += 2;
		debug_assert!(stack_off <= total_size);
	}

	/// TODO doc
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
			let interpreter_path_str = str::from_utf8(interpreter_path).or(Err(errno::EINVAL))?;
			let interpreter_path = Path::from_string(interpreter_path_str, true)?;
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
		let kernel_stack = mem_space.map_stack(None, KERNEL_STACK_SIZE, KERNEL_STACK_FLAGS)?;
		// The user stack
		let user_stack = mem_space.map_stack(None, USER_STACK_SIZE, USER_STACK_FLAGS)?;

		// The size in bytes of the initial data on the stack
		let total_size = Self::get_init_stack_size(argv, envp).1;
		// The number of pages to allocate on the user stack to write the initial data
		let pages_count = math::ceil_division(total_size, memory::PAGE_SIZE);
		// Checking that the data doesn't exceed the stack's size
		if pages_count >= USER_STACK_SIZE {
			return Err(errno::ENOMEM);
		}

		// Allocating the pages on the stack to write the initial data
		for i in 0..pages_count {
			mem_space.alloc((user_stack as usize - (i + 1) * memory::PAGE_SIZE) as *const c_void)?;
		}

		mem_space.get_vmem().flush();

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
			self.init_stack(user_stack, argv, envp);
		});

		// Setting the new memory space to the process
		process.set_mem_space(Some(mem_space));

		// Setting the process's stacks
		process.user_stack = Some(user_stack);
		process.kernel_stack = Some(kernel_stack);

		// Resetting signals
		process.signals_bitfield.clear_all();
		process.signal_handlers = [SignalHandler::Default; signal::SIGNALS_COUNT + 1];

		// TODO Enable floats and SSE

		// The pointer to the bottom of the stack
		let stack_bottom = (user_stack as usize - total_size) as _;

		// Setting the process's entry point
		let hdr = parser.get_header();
		process.regs = Regs {
			ebp: 0x0,
			esp: stack_bottom,
			eip: load_base as u32 + hdr.e_entry,
			eflags: process::DEFAULT_EFLAGS,
			eax: 0x0,
			ebx: 0x0,
			ecx: 0x0,
			edx: 0x0,
			esi: 0x0,
			edi: 0x0,
		};

		Ok(())
	}
}
