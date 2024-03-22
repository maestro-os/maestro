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

use crate::{
	elf,
	elf::{
		parser::ELFParser,
		relocation::{ELF32Rel, ELF32Rela, Relocation, GOT_SYM},
		ELF32Sym,
	},
};
use core::{
	cmp::min,
	mem::{size_of, transmute},
	slice,
};
use utils::{
	collections::{hashmap::HashMap, string::String, vec::Vec},
	errno,
	errno::EResult,
	lock::Mutex,
	vec, DisplayableStr, TryClone,
};
use version::{Dependency, Version};

/// The magic number that must be present inside of a module.
pub const MOD_MAGIC: u64 = 0x9792df56efb7c93f;

/// Macro used to declare a kernel module.
///
/// This macro must be used only inside of a kernel module.
///
/// The argument is the list of dependencies ([`Dependency`]) of the module.
///
/// Example:
/// ```rust
/// kernel::module!([Dependency {
/// 	name: "plop",
/// 	version: Version::new(1, 0, 0),
/// 	constraint: Ordering::Equal,
/// }])
/// ```
#[macro_export]
macro_rules! module {
	($deps:expr) => {
		mod module_meta {
			use kernel::module::version::Dependency;
			use kernel::module::version::Version;

			const fn get_version() -> Version {
				let result = Version::parse(env!("CARGO_PKG_VERSION"));
				let Some(version) = result else {
					panic!("invalid module version (see kernel's documentation for versioning specifications)");
				};
				version
			}

			const fn const_len<const C: usize>(_: &[Dependency; C]) -> usize {
				C
			}

			#[no_mangle]
			pub static MOD_MAGIC: u64 = kernel::module::MOD_MAGIC;

			#[no_mangle]
			pub static MOD_NAME: &'static str = env!("CARGO_PKG_NAME");

			#[no_mangle]
			pub static MOD_VERSION: Version = get_version();

			#[no_mangle]
			pub static MOD_DEPS: [Dependency; const_len(&$deps)] = $deps;
		}
	};
}

// TODO keep offsets of name, version and dependencies instead of allocating
/// Structure representing a kernel module.
pub struct Module {
	/// The module's name.
	name: String,
	/// The module's version.
	version: Version,

	/// The list of dependencies associated with the module.
	deps: Vec<Dependency>,

	/// The module's memory.
	mem: Vec<u8>,
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
		// The symbol on the kernel side
		let kernel_sym = elf::kernel::get_symbol_by_name(name)?;
		// TODO check symbols from other loaded modules
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
	fn get_attribute<'mem, T>(
		mem: &'mem [u8],
		parser: &ELFParser<'mem>,
		name: &[u8],
	) -> Option<&'mem T> {
		let sym = parser.get_symbol_by_name(name)?;
		let off = sym.st_value as usize;
		if off + size_of::<T>() >= mem.len() {
			return None;
		}
		let val = unsafe { &*(&mem[off] as *const _ as *const T) };
		Some(val)
	}

	/// Returns the array value of the given attribute of a module.
	///
	/// Arguments:
	/// - `mem` is the segment of memory on which the module is loaded.
	/// - `parser` is the module's parser.
	/// - `name` is the attribute's name.
	///
	/// If the attribute doesn't exist, the function returns `None`.
	fn get_array_attribute<'mem, T>(
		mem: &'mem [u8],
		parser: &ELFParser<'mem>,
		name: &[u8],
	) -> Option<&'mem [T]> {
		let sym = parser.get_symbol_by_name(name)?;
		let off = sym.st_value as usize;
		let len = sym.st_size as usize / size_of::<T>();
		let slice = unsafe {
			let ptr = &*(&mem[off] as *const _ as *const T);
			slice::from_raw_parts(ptr, len)
		};
		Some(slice)
	}

	/// Loads a kernel module from the given image.
	pub fn load(image: &[u8]) -> EResult<Self> {
		let parser = ELFParser::new(image).map_err(|e| {
			crate::println!("Invalid ELF file as loaded module");
			e
		})?;

		// Allocate memory for the module
		let mem_size = Self::get_load_size(&parser);
		let mut mem = vec![0; mem_size]?;

		// The base virtual address at which the module is loaded
		let load_base = mem.as_ptr();

		// Copying the module's image
		parser
			.iter_segments()
			.filter(|seg| seg.p_type != elf::PT_NULL)
			.for_each(|seg| {
				let len = min(seg.p_memsz, seg.p_filesz) as usize;
				let mem_begin = seg.p_vaddr as usize;
				let image_begin = seg.p_offset as usize;
				mem.as_mut_slice()[mem_begin..(mem_begin + len)]
					.copy_from_slice(&image[image_begin..(image_begin + len)]);
			});

		// Closure returning a symbol
		let get_sym = |sym_section: u32, sym: u32| {
			let section = parser.get_section_by_index(sym_section as _)?;
			let sym = parser.get_symbol_by_index(section, sym as _)?;

			if !sym.is_defined() {
				let strtab = parser.get_section_by_index(section.sh_link as _)?;
				let name = parser.get_symbol_name(strtab, sym)?;

				// Look inside the kernel image or other modules
				let Some(other_sym) = Self::resolve_symbol(name) else {
					crate::println!(
						"Symbol `{}` not found in kernel or other loaded modules",
						DisplayableStr(name)
					);
					return None;
				};
				Some(other_sym.st_value)
			} else {
				Some(load_base as u32 + sym.st_value)
			}
		};

		let got_sym = parser.get_symbol_by_name(GOT_SYM);
		for section in parser.iter_sections() {
			for rel in parser.iter_rel::<ELF32Rel>(section) {
				unsafe { rel.perform(load_base as _, section, get_sym, got_sym) }
					.map_err(|_| errno!(EINVAL))?;
			}
			for rela in parser.iter_rel::<ELF32Rela>(section) {
				unsafe { rela.perform(load_base as _, section, get_sym, got_sym) }
					.map_err(|_| errno!(EINVAL))?;
			}
		}

		// Checking the magic number
		let magic = Self::get_attribute::<u64>(mem.as_slice(), &parser, b"MOD_MAGIC").ok_or_else(
			|| {
				crate::println!("Missing `MOD_MAGIC` symbol in module image");
				errno!(EINVAL)
			},
		)?;
		if *magic != MOD_MAGIC {
			crate::println!("Module has an invalid magic number");
			return Err(errno!(EINVAL));
		}

		// Getting the module's name
		let name = Self::get_attribute::<&'static str>(mem.as_slice(), &parser, b"MOD_NAME")
			.ok_or_else(|| {
				crate::println!("Missing `MOD_NAME` symbol in module image");
				errno!(EINVAL)
			})?;
		let name = String::try_from(*name)?;

		// Getting the module's version
		let version = Self::get_attribute::<Version>(mem.as_slice(), &parser, b"MOD_VERSION")
			.ok_or_else(|| {
				crate::println!("Missing `MOD_VERSION` symbol in module image");
				errno!(EINVAL)
			})?;

		// Getting the module's dependencies
		let deps = Self::get_array_attribute::<Dependency>(mem.as_slice(), &parser, b"MOD_DEPS")
			.ok_or_else(|| {
			crate::println!("Missing `MOD_DEPS` symbol in module image");
			errno!(EINVAL)
		})?;
		let deps = Vec::from_slice(deps)?;

		crate::println!("Loading module `{name}` version `{version}`");

		// TODO Check that all dependencies are loaded

		// Initializing module
		let init = parser.get_symbol_by_name(b"init").ok_or_else(|| {
			crate::println!("Missing `init` symbol in module image");
			errno!(EINVAL)
		})?;
		let ok = unsafe {
			let ptr = mem.as_ptr().add(init.st_value as usize);
			let func: extern "C" fn() -> bool = transmute(ptr);
			(func)()
		};
		if !ok {
			crate::println!("Failed to load module `{name}`");
			return Err(errno!(EINVAL));
		}

		// Retrieving destructor function
		let fini = parser.get_symbol_by_name(b"fini").map(|fini| unsafe {
			let ptr = mem.as_ptr().add(fini.st_value as usize);
			let func: extern "C" fn() = transmute(ptr);
			func
		});

		Ok(Self {
			name,
			version: *version,

			deps,

			mem: mem as _,
			mem_size,

			fini,
		})
	}

	/// Returns the name of the module.
	pub fn get_name(&self) -> &[u8] {
		&self.name
	}

	/// Returns the [`Version`] of the module.
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
pub fn add(module: Module) -> EResult<()> {
	let mut modules = MODULES.lock();
	modules.insert(module.name.try_clone()?, module)?;
	Ok(())
}

/// Removes the module with name `name`.
pub fn remove(name: &[u8]) {
	let mut modules = MODULES.lock();
	modules.remove(name);
}
