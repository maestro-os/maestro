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
use crate::elf::ELF32Sym;
use crate::elf::parser::ELFParser;
use crate::elf::relocation::Relocation;
use crate::elf;
use crate::errno::Errno;
use crate::errno;
use crate::memory::malloc;
use crate::memory;
use crate::multiboot;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;
use crate::util::lock::Mutex;
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

	/// Resolves an external symbol from the kernel or another module. If the symbol doesn't exist,
	/// the function returns None.
	/// `name` is the name of the symbol to look for.
	fn resolve_symbol(name: &[u8]) -> Option<&ELF32Sym> {
		let boot_info = multiboot::get_boot_info();
		// The symbol on the kernel side
		let kernel_sym = elf::get_kernel_symbol(memory::kern_to_virt(boot_info.elf_sections),
			boot_info.elf_num as usize, boot_info.elf_shndx as usize,
			boot_info.elf_entsize as usize, name)?;

		// TODO Check other modules
		Some(kernel_sym)
	}

	// TODO Print a warning when a symbol cannot be resolved
	/// Loads a kernel module from the given image.
	pub fn load(image: &[u8]) -> Result<Self, Errno> {
		let parser = ELFParser::new(image).or_else(| e | {
			crate::println!("Failed to parse module file");
			Err(e)
		})?;

		// TODO Read and check the magic number

		// Allocating memory for the module
		let mem_size = Self::get_load_size(&parser);
		let mut mem = malloc::Alloc::<u8>::new_default(mem_size)?;

		// The base virtual address at which the module is loaded
		let load_base = unsafe {
			mem.as_ptr() as u32
		};

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

		// Closure returning a symbol from its name
		let get_sym = | name: &str | parser.get_symbol_by_name(name);

		// Closure returning the value of the given symbol
		let get_sym_val = | sym_section: u32, sym: u32 | {
			let section = parser.get_section_by_index(sym_section)?;
			let sym = parser.get_symbol_by_index(section, sym)?;

			if !sym.is_defined() {
				let strtab = parser.get_section_by_index(section.sh_link)?;

				// Looking inside of the kernel image or other modules
				let name = parser.get_symbol_name(strtab, sym)?;
				let other_sym = Self::resolve_symbol(name)?;
				Some(other_sym.st_value)
			} else {
				Some(load_base + sym.st_value)
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

		// Getting the module's name
		let mod_name = parser.get_symbol_by_name("mod_name").or_else(|| {
			crate::println!("Missing `mod_name` symbol in module image");
			None
		}).ok_or(errno::EINVAL)?;
		let name_str = unsafe {
			let ptr = mem.as_ptr().add(mod_name.st_value as usize);
			let func: extern "C" fn() -> &'static str = transmute(ptr);
			(func)()
		};
		let name = String::from(name_str.as_bytes())?;

		// Getting the module's version
		let mod_version = parser.get_symbol_by_name("mod_version").or_else(|| {
			crate::println!("Missing `mod_version` symbol in module image");
			None
		}).ok_or(errno::EINVAL)?;
		let version = unsafe {
			let ptr = mem.as_ptr().add(mod_version.st_value as usize);
			let func: extern "C" fn() -> Version = transmute(ptr);
			(func)()
		};

		crate::println!("Loading module `{}` version {}", name, version);

		// Initializing module
		let init = parser.get_symbol_by_name("init").or_else(|| {
			crate::println!("Missing `init` symbol in module image");
			None
		}).ok_or(errno::EINVAL)?;
		let ok = unsafe {
			let ptr = mem.as_ptr().add(init.st_value as usize);
			let func: extern "C" fn() -> bool = transmute(ptr);
			(func)()
		};
		if !ok {
			crate::println!("Failed to load module `{}`", name);
			return Err(errno::EINVAL);
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
		if let Some(fini) = self.fini {
			fini();
		}

		crate::println!("Unloaded module `{}`", self.name);
	}
}

/// The list of modules.
static MODULES: Mutex<Vec<Module>> = Mutex::new(Vec::new());

// TODO Optimize
/// Tells whether a module with the given name is loaded.
pub fn is_loaded(name: &String) -> bool {
	let modules_guard = MODULES.lock();
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
	let mut modules_guard = MODULES.lock();
	let modules = modules_guard.get_mut();
	modules.push(module)
}

// TODO Function to remove a module
