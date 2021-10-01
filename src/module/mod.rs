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
use core::mem::transmute;
use core::ptr;
use crate::elf::ELFParser;
use crate::elf::Relocation;
use crate::elf;
use crate::errno::Errno;
use crate::errno;
use crate::memory::malloc;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;
use crate::util::lock::mutex::Mutex;
use version::Version;

/// The magic number that must be present inside of a module.
pub const MODULE_MAGIC: u64 = 0x9792df56efb7c93f;

// TODO Add a symbol containing the magic number

/// Macro used to declare a kernel module. This macro must be used only inside of a kernel module.
/// `name` (str) is the module's name.
/// `version` (Version) is the module's version.
#[macro_export]
macro_rules! module {
	($name:expr, $version:expr) => {
		#[no_mangle]
		pub extern "C" fn mod_name() -> &'static str {
			$name
		}

		#[no_mangle]
		pub extern "C" fn mod_version() -> kernel::module::version::Version {
			$version
		}
	}
}

/// Structure representing a kernel module.
pub struct Module {
	/// The module's name.
	name: String,
	/// The module's version.
	version: Version,

	// TODO Add dependencies handling

	/// The module's memory.
	mem: malloc::Alloc::<u8>,
	/// The size of the module's memory.
	mem_size: usize,

	/// Pointer to the module's destructor.
	fini: Option<extern "C" fn()>,
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

	// TODO On fail, print the reason in kernel logs
	/// Loads a kernel module from the given image.
	pub fn load(image: &[u8]) -> Result<Self, Errno> {
		let parser = ELFParser::new(image)?;

		// TODO Read and check the magic number

		// Allocating memory for the module
		let mem_size = Self::get_load_size(&parser);
		let mut mem = malloc::Alloc::<u8>::new_default(mem_size)?;

		// Copying the module's image
		parser.foreach_segments(| seg | {
			if seg.p_type != elf::PT_NULL {
				let len = min(seg.p_memsz, seg.p_filesz) as usize;
				unsafe { // Safe because the module ELF image is valid
					ptr::copy_nonoverlapping(&image[seg.p_offset as usize],
						&mut mem.get_slice_mut()[seg.p_vaddr as usize],
						len);
				}
			}

			true
		});

		// TODO Fill GOT

		// TODO Move somewhere else
		// Closure performing a relocation.
		// TODO doc arguments
		let perform_reloc = | offset: u32, _sym: u32, type_: u8, addend: u32 | {
			// The virtual address at which the image is located
			let base_addr = unsafe {
				mem.as_ptr() as u32
			};
			// The offset inside of the GOT
			let got_offset = 0; // TODO
			// The address of the GOT
			let got_addr = 0; // TODO
			// The offset of the PLT entry for the symbol.
			let plt_offset = 0; // TODO
			// The value of the symbol
			let sym_val = 0; // TODO

			let offset = match type_ {
				elf::R_386_32 => Some(sym_val + addend),
				elf::R_386_PC32 => Some(sym_val + addend - offset),
				elf::R_386_GOT32 => Some(got_offset + addend),
				elf::R_386_PLT32 => Some(plt_offset + addend - offset),
				elf::R_386_GLOB_DAT | elf::R_386_JMP_SLOT => Some(sym_val),
				elf::R_386_RELATIVE => Some(base_addr + addend),
				elf::R_386_GOTOFF => Some(sym_val + addend - got_addr),
				elf::R_386_GOTPC => Some(got_addr + addend - offset),

				_ => None,
			};

			if let Some(_offset) = offset {
				// TODO Perform relocation
			}
		};

		parser.foreach_rel(| rel | {
			perform_reloc(rel.r_offset, rel.get_sym(), rel.get_type(), 0);
			true
		});
		parser.foreach_rela(| rela | {
			perform_reloc(rela.r_offset, rela.get_sym(), rela.get_type(), rela.r_addend);
			true
		});

		// Getting the module's name
		let mod_name = parser.get_symbol_by_name("mod_name").ok_or(errno::EINVAL)?;
		let name_str = unsafe {
			let ptr = mem.as_ptr().add(mod_name.st_value as usize);
			let func: extern "C" fn() -> &'static str = transmute(ptr);
			(func)()
		};
		let name = String::from(name_str)?;

		// Getting the module's version
		let mod_version = parser.get_symbol_by_name("mod_version").ok_or(errno::EINVAL)?;
		let version = unsafe {
			let ptr = mem.as_ptr().add(mod_version.st_value as usize);
			let func: extern "C" fn() -> Version = transmute(ptr);
			(func)()
		};

		// Initializing module
		let init = parser.get_symbol_by_name("init").ok_or(errno::EINVAL)?;
		unsafe {
			let ptr = mem.as_ptr().add(init.st_value as usize);
			let func: extern "C" fn() -> Version = transmute(ptr);
			(func)();
		}

		// Retrieving destructor function
		let fini = {
			if let Some(fini) = parser.get_symbol_by_name("fini") {
				unsafe {
					let ptr = mem.as_ptr().add(fini.st_value as usize);
					let func: extern "C" fn() = transmute(ptr);
					Some(func)
				}
			} else {
				None
			}
		};

		crate::println!("Loaded module `{}` version {}", name, version);
		Ok(Self {
			name,
			version,

			mem: mem as _,
			mem_size,

			fini,
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
		crate::println!("Unloaded module `{}`", self.name);

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
