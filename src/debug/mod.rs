///TODO doc

use core::ffi::c_void;
use crate::elf;
use crate::memory;
use crate::multiboot;
use crate::util;

/// Returns the value into the specified register.
#[macro_export]
macro_rules! register_get {
	($reg:expr) => {{
		let mut val: u32;
		llvm_asm!(concat!("mov %", $reg, ", %eax") : "={eax}"(val));

		val
	}};
}

/// Prints the registers into the given `regs` structure.
pub fn print_regs(regs: &util::Regs) {
	crate::print!("ebp: {:p} ", regs.ebp as *const c_void);
	crate::print!("esp: {:p} ", regs.esp as *const c_void);
	crate::print!("eip: {:p} ", regs.eip as *const c_void);
	crate::print!("eflags: {:p} ", regs.eflags as *const c_void);
	crate::print!("eax: {:p}\n", regs.eax as *const c_void);
	crate::print!("ebx: {:p} ", regs.ebx as *const c_void);
	crate::print!("ecx: {:p} ", regs.ecx as *const c_void);
	crate::print!("edx: {:p} ", regs.edx as *const c_void);
	crate::print!("esi: {:p} ", regs.esi as *const c_void);
	crate::print!("edi: {:p}\n", regs.edi as *const c_void);
}

/// Prints, in hexadecimal, the content of the memory at the given location `ptr`, with the given
/// size `n` in bytes.
pub unsafe fn print_memory(ptr: *const c_void, n: usize) {
	let mut i = 0;
	while i < n {
		crate::print!("{:p}  ", ptr);

		let mut j = 0;
		while j < 16 && i + j < n {
			crate::println!("{:x?} ", *(((ptr as usize) + (i + j)) as *const u8));
			j += 1;
		}

		crate::print!(" |");

		j = 0;
		while j < 16 && i + j < n {
			let v = *(((ptr as usize) + (i + j)) as *const u8);
			let c = if v < 32 {
				'.'
			} else {
				v as char
			};
			crate::println!("{}", c);
			j += 1;
		}

		crate::println!("|");

		i += j;
	}
}

/// Returns the name of the symbol at offset `offset`.
fn get_symbol_name(offset: u32) -> Option<&'static str> {
	let boot_info = multiboot::get_boot_info();

	if let Some(section) = elf::get_section(boot_info.elf_sections, boot_info.elf_num as usize,
		boot_info.elf_shndx as usize, boot_info.elf_entsize as usize, ".strtab") {
		let name = unsafe {
			util::ptr_to_str(memory::kern_to_virt((section.sh_addr + offset) as _))
		};
		Some(name)
	} else {
		None
	}
}

/// Returns an Option containing the name of the function for the given instruction pointer. If the
/// name cannot be retrieved, the function returns None.
fn get_function_name(inst: *const c_void) -> Option<&'static str> {
	if inst < memory::get_kernel_virtual_begin() || inst >= memory::get_kernel_virtual_end() {
		return None;
	}

	let boot_info = multiboot::get_boot_info();
	let mut func_name: Option<&'static str> = None;
	elf::foreach_sections(boot_info.elf_sections, boot_info.elf_num as usize,
		boot_info.elf_shndx as usize, boot_info.elf_entsize as usize,
		|hdr: &elf::ELF32SectionHeader, _name: &str| {
			if hdr.sh_type != elf::SHT_SYMTAB {
				return;
			}

			let ptr = memory::kern_to_virt(hdr.sh_addr as _);
			let mut i: usize = 0;
			while i < hdr.sh_size as usize {
				let sym = unsafe {
					&*(ptr.offset(i as isize) as *const elf::ELF32Sym)
				};
				let value = sym.st_value as usize;
				//let size = sym.st_size as usize;

				// TODO Fix overflow
				if (inst as usize) >= value/* && (inst as usize) < (value + size)*/ {
					if sym.st_name != 0 {
						func_name = get_symbol_name(sym.st_name);
					}
					return;
				}
				i += core::mem::size_of::<elf::ELF32Sym>();
			}
		});
	func_name
}

/// Prints the callstack in the current context, including symbol's name and address. `ebp` is
/// value of the `%ebp` register that is used as a starting point for printing. `max_depth` is the
/// maximum depth of the stack to print. If the stack is larger than the maximum depth, the
/// function shall print `...` at the end. If the callstack is empty, the function just prints
/// `Empty`.
pub fn print_callstack(ebp: *const u32, max_depth: usize) {
	crate::println!("--- Callstack ---");

	let mut i: usize = 0;
	let mut ebp_ = ebp;
	while ebp_ != 0 as *const u32 && i < max_depth {
		// TODO
		/*if !memory::vmem::is_mapped(memory::kern_to_virt(memory::cr3_get()), ebp_) {
			break;
		}*/
		let eip = unsafe {
			*((ebp_ as usize + core::mem::size_of::<usize>()) as *const u32) as *const _
		};
		if eip == (0 as *const _) {
			break;
		}

		if let Some(name) = get_function_name(eip) {
			crate::println!("{}: {:p} -> {}", i, eip, name);
		} else {
			crate::println!("{}: {:p} -> ???", i, eip);
		}

		unsafe {
			ebp_ = *(ebp_ as *const u32) as *const u32;
		}
		i += 1;
	}

	if i == 0 {
		crate::println!("Empty");
	} else if ebp_ != (0 as *const _) {
		crate::println!("...");
	}
}
