//! This module implements ELF program execution with respects the System V ABI.

use core::cmp::min;
use core::ffi::c_void;
use core::ptr;
use core::slice;
use crate::elf::parser::ELFParser;
use crate::elf::relocation::Relocation;
use crate::elf;
use crate::errno::Errno;
use crate::file::path::Path;
use crate::file;
use crate::memory::malloc;
use crate::memory::vmem;
use crate::memory;
use crate::process::Process;
use crate::process::exec::Executor;
use crate::process;
use crate::util::Regs;
use crate::util::math;

/// The program executor for ELF files.
pub struct ELFExecutor {
	/// The program image.
	image: malloc::Alloc<u8>,
}

impl ELFExecutor {
	/// Creates a new instance to execute the given program.
	/// `path` is the path to the program.
	pub fn new(path: &Path) -> Result<Self, Errno> {
		// Reading the file's content
		let image = {
			let mutex = file::get_files_cache();
			let mut guard = mutex.lock(true);
			let files_cache = guard.get_mut();

			let mut file_mutex = files_cache.get_file_from_path(&path)?;
			let file_lock = file_mutex.lock(true);
			let file = file_lock.get();

			let len = file.get_size();
			let mut image = unsafe {
				malloc::Alloc::new_zero(len as usize)?
			};
			file.read(0, image.get_slice_mut())?;

			image
		};

		Ok(Self {
			image,
		})
	}

	// TODO Clean
	// TODO Ensure the stack capacity is not exceeded
	/// Initializes the stack data of the process according to the System V ABI.
	/// `user_stack` is the pointer to the top of the user stack.
	/// `argv` is the list of arguments.
	/// `envp` is the environment.
	fn init_stack(&self, user_stack: *const c_void, argv: &[&str], envp: &[&str]) {
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

		// A slice on the stack representing the region to fill
		let stack_slice = unsafe {
			slice::from_raw_parts_mut((user_stack as usize - total_size) as *mut u32,
				total_size / 4)
		};
		// A slice on the stack representing the information block
		let info_slice = unsafe {
			slice::from_raw_parts_mut((user_stack as usize - info_block_size) as *mut u8,
				info_block_size)
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

			debug_assert!(info_off < info_block_size);

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

			debug_assert!(info_off < info_block_size);

			// Setting the variable's pointer
			stack_slice[stack_off] = &mut info_slice[begin] as *mut _ as u32;
			stack_off += 1;
		}
		// Setting the nullbytes to end envp
		for i in 0..2 {
			stack_slice[stack_off + i] = 0;
		}
		stack_off += 2;
		debug_assert!(stack_off < stack_slice.len());
	}
}

impl Executor for ELFExecutor {
	// TODO Ensure there is no way to write in kernel space (check segments position and
	// relocations)
	fn exec(&self, process: &mut Process, argv: &[&str], envp: &[&str]) -> Result<(), Errno> {
		debug_assert_eq!(process.state, crate::process::State::Running);

		// Parsing the ELF file
		let parser = ELFParser::new(self.image.get_slice())?;

		// The top of the user stack
		let user_stack = process.user_stack;
		// The base at which the program is loaded
		let load_base = memory::PAGE_SIZE as *mut u8; // TODO Support ASLR

		// The current process's memory space
		debug_assert!(process.mem_space.is_some());
		let mem_space = process.mem_space.as_mut().unwrap();

		// Allocating memory for segments
		parser.foreach_segments(| seg | {
			if seg.p_type != elf::PT_NULL {
				let len = min(seg.p_memsz, seg.p_filesz) as usize;
				let _pages = math::ceil_division(len, memory::PAGE_SIZE);
				// TODO Map in mem_space
			}

			true
		});
		mem_space.get_vmem().flush();

		vmem::switch(mem_space.get_vmem().as_ref(), || {
			parser.foreach_segments(| seg | {
				if seg.p_type != elf::PT_NULL {
					let len = min(seg.p_memsz, seg.p_filesz) as usize;
					unsafe { // Safe because the module ELF image is valid
						ptr::copy_nonoverlapping(&self.image[seg.p_offset as usize],
							load_base.add(seg.p_vaddr as usize),
							len);
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

			self.init_stack(user_stack, argv, envp);
		});

		// TODO Reset signals, etc...

		// TODO Enable floats and SSE

		// Setting the process's entry point
		let hdr = parser.get_header();
		process.regs = Regs {
			ebp: 0x0,
			esp: process.user_stack as _,
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
