//! A kernel module is an executable file that is loaded in kernelspace in order to
//! handle a specific feature such as device drivers.
//!
//! The some advantages of that system is a lighter kernel with clearer code and it allows to only
//! load subsystems that are currently required.
//!
//! There's a distinction between a **Module** and a **Kernel Module**:
//! - **Module**: A *Rust* module, part of the structure of the code.
//! - **Kernel Module**: A piece of software to be loaded at runtime in kernelspace.
//!
//! Thus, **Kernel Modules** contain **Modules**.

pub mod version;

use crate::elf;
use crate::elf::parser::ELFParser;
use crate::elf::relocation::Relocation;
use crate::elf::ELF32Sym;
use crate::errno;
use crate::errno::Errno;
use crate::memory;
use crate::memory::malloc;
use crate::multiboot;
use crate::util::container::hashmap::HashMap;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;
use crate::util::lock::Mutex;
use crate::util::DisplayableStr;
use crate::util::TryClone;
use core::cmp::min;
use core::mem::size_of;
use core::mem::transmute;
use core::ptr;
use version::Dependency;
use version::Version;

/// The magic number that must be present inside of a module.
pub const MOD_MAGIC: u64 = 0x9792df56efb7c93f;

/// Macro used to declare a kernel module.
///
/// This macro must be used only inside of a kernel module.
///
/// Arguments:
/// - `name` (str) is the module's name.
/// - `version` ([`version::Version`]) is the module's version.
/// - `deps` ([&str]) is the list of the module's dependencies.
#[macro_export]
macro_rules! module {
	($name:expr, $version:expr, $deps:expr) => {
		#[no_mangle]
		pub static MOD_MAGIC: u64 = kernel::module::MOD_MAGIC;

		#[no_mangle]
		pub static MOD_NAME: &'static str = $name;

		#[no_mangle]
		pub static MOD_VERSION: kernel::module::version::Version = $version;

		#[no_mangle]
		pub static MOD_DEPS: &'static [kernel::module::version::Dependency] = $deps;
	};
}

/// Structure representing a kernel module.
pub struct Module {
	/// The module's name.
	name: String,
	/// The module's version.
	version: Version,

	/// The list of dependencies associated with the module.
	deps: Vec<Dependency>,

	/// The module's memory.
	mem: malloc::Alloc<u8>,
	/// The size of the module's memory.
	mem_size: usize,

	/// Pointer to the module's destructor.
	fini: Option<extern "C" fn()>,
}

impl Module {
	/// Returns the size required to load the module image.
	fn get_load_size(parser: &ELFParser) -> usize {
		parser
			.iter_segments()
			.map(|seg| seg.p_vaddr as usize + seg.p_memsz as usize)
			.max()
			.unwrap_or(0)
	}

	/// Resolves an external symbol from the kernel or another module.
	///
	/// `name` is the name of the symbol to look for.
	///
	/// If the symbol doesn't exist, the function returns `None`.
	fn resolve_symbol(name: &[u8]) -> Option<&ELF32Sym> {
		let boot_info = multiboot::get_boot_info();
		// The symbol on the kernel side
		let kernel_sym = elf::get_kernel_symbol(
			memory::kern_to_virt(boot_info.elf_sections),
			boot_info.elf_num as usize,
			boot_info.elf_shndx as usize,
			boot_info.elf_entsize as usize,
			name,
		)?;

		// TODO Check other modules
		Some(kernel_sym)
	}

	/// Returns the value of the given attribute of a module.
	///
	/// Arguments:
	/// - `mem` is the segment of memory on which the module is loaded.
	/// - `parser` is the module's parser.
	/// - `name` is the attribute's name.
	///
	/// If the attribute doesn't exist, the function returns `None`.
	fn get_module_attibute<'a, T>(mem: &'a [u8], parser: &ELFParser<'a>, name: &str) -> Option<T> {
		let sym = parser.get_symbol_by_name(name)?;

		let off = sym.st_value as usize;
		if off >= mem.len() || off + size_of::<T>() >= mem.len() {
			return None;
		}

		let val = unsafe {
			let ptr = mem.as_ptr().add(off) as *const T;
			ptr::read(ptr)
		};

		Some(val)
	}

	/// Loads a kernel module from the given image.
	pub fn load(image: &[u8]) -> Result<Self, Errno> {
		let parser = ELFParser::new(image).map_err(|e| {
			crate::println!("Invalid ELF file as loaded module");
			e
		})?;

		// Allocating memory for the module
		let mem_size = Self::get_load_size(&parser);
		let mut mem = malloc::Alloc::<u8>::new_default(mem_size)?;

		// The base virtual address at which the module is loaded
		let load_base = unsafe { mem.as_ptr() as u32 };

		// Copying the module's image
		parser
			.iter_segments()
			.filter(|seg| seg.p_type != elf::PT_NULL)
			.for_each(|seg| {
				let len = min(seg.p_memsz, seg.p_filesz) as usize;

				unsafe {
					// Safe because the module ELF image is valid
					ptr::copy_nonoverlapping(
						&image[seg.p_offset as usize],
						&mut mem.as_slice_mut()[seg.p_vaddr as usize],
						len,
					);
				}
			});

		// Closure returning a symbol from its name
		let get_sym = |name: &str| parser.get_symbol_by_name(name);

		// Closure returning the value of the given symbol
		let get_sym_val = |sym_section: u32, sym: u32| {
			let section = parser.iter_sections().nth(sym_section as usize)?;
			let sym = parser.iter_symbols(section).nth(sym as usize)?;

			if !sym.is_defined() {
				let strtab = parser.iter_sections().nth(section.sh_link as usize)?;
				let name = parser.get_symbol_name(strtab, sym)?;

				// Looking inside of the kernel image or other modules
				let Some(other_sym) = Self::resolve_symbol(name) else {
					crate::println!(
						"Symbol `{}` not found in kernel or other loaded modules",
						DisplayableStr(name)
					);
					return None;
				};

				Some(other_sym.st_value)
			} else {
				Some(load_base + sym.st_value)
			}
		};

		for section in parser.iter_sections() {
			for rel in parser.iter_rel(section) {
				unsafe { rel.perform(load_base as _, section, get_sym, get_sym_val) }
					.map_err(|_| errno!(EINVAL))?;
			}

			for rela in parser.iter_rela(section) {
				unsafe { rela.perform(load_base as _, section, get_sym, get_sym_val) }
					.map_err(|_| errno!(EINVAL))?;
			}
		}

		// Checking the magic number
		let magic = Self::get_module_attibute::<u64>(mem.as_slice(), &parser, "MOD_MAGIC")
			.ok_or_else(|| {
				crate::println!("Missing `MOD_MAGIC` symbol in module image");
				errno!(EINVAL)
			})?;
		if magic != MOD_MAGIC {
			crate::println!("Module has an invalid magic number");
			return Err(errno!(EINVAL));
		}

		// Getting the module's name
		let name = Self::get_module_attibute::<&'static str>(mem.as_slice(), &parser, "MOD_NAME")
			.ok_or_else(|| {
				crate::println!("Missing `MOD_NAME` symbol in module image");
				errno!(EINVAL)
			})?;
		let name = String::try_from(name)?;

		// Getting the module's version
		let version = Self::get_module_attibute::<Version>(mem.as_slice(), &parser, "MOD_VERSION")
			.ok_or_else(|| {
				crate::println!("Missing `MOD_VERSION` symbol in module image");
				errno!(EINVAL)
			})?;

		// Getting the module's dependencies
		let deps = Self::get_module_attibute::<&'static [Dependency]>(
			mem.as_slice(),
			&parser,
			"MOD_DEPS",
		)
		.ok_or_else(|| {
			crate::println!("Missing `MOD_DEPS` symbol in module image");
			errno!(EINVAL)
		})?;
		let deps = Vec::from_slice(deps)?;

		crate::println!("Loading module `{}` version {}", name, version);

		// TODO Check that all dependencies are loaded

		// Initializing module
		let init = parser.get_symbol_by_name("init").ok_or_else(|| {
			crate::println!("Missing `init` symbol in module image");
			errno!(EINVAL)
		})?;
		let ok = unsafe {
			let ptr = mem.as_ptr().add(init.st_value as usize);
			let func: extern "C" fn() -> bool = transmute(ptr);

			(func)()
		};
		if !ok {
			crate::println!("Failed to load module `{}`", name);
			return Err(errno!(EINVAL));
		}

		// Retrieving destructor function
		let fini = {
			if let Some(fini) = parser.get_symbol_by_name("fini") {
				let fini = unsafe {
					let ptr = mem.as_ptr().add(fini.st_value as usize);
					let func: extern "C" fn() = transmute(ptr);

					func
				};

				Some(fini)
			} else {
				None
			}
		};

		Ok(Self {
			name,
			version,

			deps,

			mem: mem as _,
			mem_size,

			fini,
		})
	}

	/// Returns the name of the module.
	pub fn get_name(&self) -> &String {
		&self.name
	}

	/// Returns the [`version::Version`] of the module.
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

/// The list of modules. The key is the name of the module and the value is the
/// module itself.
static MODULES: Mutex<HashMap<String, Module>> = Mutex::new(HashMap::new());

/// Tells whether a module with the given name is loaded.
pub fn is_loaded(name: &[u8]) -> bool {
	let modules = MODULES.lock();
	modules.get(name).is_some()
}

/// Adds the given module to the modules list.
pub fn add(module: Module) -> Result<(), Errno> {
	let mut modules = MODULES.lock();
	modules.insert(module.name.try_clone()?, module)?;

	Ok(())
}

/// Removes the module with name `name`.
pub fn remove(name: &[u8]) {
	let mut modules = MODULES.lock();
	modules.remove(name);
}
