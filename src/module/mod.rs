//! A kernel module is an executable that is loaded in kernelspace in order to handle a specific
//! feature. The some advantages of that system is a lighter kernel with clearer code and it
//! allows to load only the parts that are required by the current system.
//!
//! There's a distinction between a Module and a Kernel module:
//! - Module: A Rust module, part of the structure of the code.
//! - Kernel module: A piece of software to be loaded at runtime in kernelspace.

pub mod version;

use core::cmp::max;
use core::cmp::min;
use core::ptr;
use crate::elf::ELFParser;
use crate::errno::Errno;
use crate::memory::malloc;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;
use crate::util::lock::mutex::Mutex;
use version::Version;

/// Structure representing a kernel module.
pub struct Module {
	/// The module's name.
	name: String,
	/// The module's version.
	version: Version,

	// TODO Add dependencies handling

	/// The module's memory.
	mem: *const u8,
	/// The size of the module's memory.
	mem_size: usize,

	/// Pointer to the module's destructor.
	fini: Option<fn()>,
}

impl Module {
	/// Returns the size required to load the module image.
	fn get_load_size(parser: &ELFParser) -> usize {
		let mut size = 0;
		parser.foreach_segments(| seg | {
			size = max(seg.p_vaddr as usize + seg.p_memsz as usize, size);
			true
		});

		size
	}

	/// Loads a kernel module from the given image.
	pub fn load(image: &[u8]) -> Result<Self, Errno> {
		let parser = ELFParser::new(image)?;
		// TODO Read and check the magic number

		let mem_size = Self::get_load_size(&parser);
		let mem = unsafe {
			malloc::alloc(mem_size)?
		} as *mut u8;

		parser.foreach_segments(| seg | {
			let len = min(seg.p_memsz, seg.p_filesz) as usize;
			unsafe { // Safe because the module ELF image is valid
				ptr::copy_nonoverlapping(&image[seg.p_offset as usize],
					mem.offset(seg.p_vaddr as _),
					len);
			}

			true
		});

		// TODO Call init function (retrieve name, version and dependencies)

		Ok(Self {
			name: String::from("TODO")?, // TODO
			version: Version {
				major: 0,
				minor: 0,
				patch: 0,
			},

			mem: mem as _,
			mem_size,

			fini: None,
		})
	}

	/// Returns the name of the module.
	pub fn get_name(&self) -> &String {
		&self.name
	}

	/// Returns the version of the module.
	pub fn get_version(&self) -> &Version {
		&self.version
	}
}

impl Drop for Module {
	fn drop(&mut self) {
		unsafe { // Safe because the pointer is valid
			malloc::free(self.mem as _);
		}

		if let Some(fini) = self.fini {
			fini();
		}
	}
}

/// The list of modules.
static mut MODULES: Mutex<Vec<Module>> = Mutex::new(Vec::new());

// TODO Optimize
/// Tells whether a module with the given name is loaded.
pub fn is_loaded(name: &String) -> bool {
	let modules_guard = unsafe { // Safe because using Mutex
		MODULES.lock(true)
	};
	let modules = modules_guard.get();

	for m in modules {
		if m.get_name() == name {
			return true;
		}
	}

	false
}

/// Adds the given module to the modules list.
pub fn add(module: Module) -> Result<(), Errno> {
	let mut modules_guard = unsafe { // Safe because using Mutex
		MODULES.lock(true)
	};
	let modules = modules_guard.get_mut();
	modules.push(module)
}

// TODO Function to remove a module
