/*
 * TODO doc
 */

use crate::elf;
use crate::memory::Void;
use crate::memory;
use crate::multiboot;
use crate::util;

/*
 * Returns the value into the specified register.
 */
#[macro_export]
macro_rules! register_get {
	($reg:expr) => {{
		let mut val: u32;
		llvm_asm!(concat!("mov %", $reg, ", %eax") : "={eax}"(val));

		val
	}};
}

/*
 * Prints the registers into the given `regs` structure.
 */
pub fn print_regs(regs: &util::Regs) {
	::print!("ebp: {:p} ", regs.ebp as *const Void);
	::print!("esp: {:p} ", regs.esp as *const Void);
	::print!("eip: {:p} ", regs.eip as *const Void);
	::print!("eflags: {:p} ", regs.eflags as *const Void);
	::print!("eax: {:p}\n", regs.eax as *const Void);
	::print!("ebx: {:p} ", regs.ebx as *const Void);
	::print!("ecx: {:p} ", regs.ecx as *const Void);
	::print!("edx: {:p} ", regs.edx as *const Void);
	::print!("esi: {:p} ", regs.esi as *const Void);
	::print!("edi: {:p}\n", regs.edi as *const Void);
}

/*
 * Prints, in hexadecimal, the content of the memory at the given location `ptr`, with the given
 * size `n` in bytes.
 */
pub unsafe fn print_memory(ptr: *const Void, n: usize) {
	let mut i = 0;
	while i < n {
		::print!("{:p}  ", ptr);

		let mut j = 0;
		while j < 16 && i + j < n {
			::println!("{:x?} ", *(((ptr as usize) + (i + j)) as *const u8));
			j += 1;
		}

		::print!(" |");

		j = 0;
		while j < 16 && i + j < n {
			let v = *(((ptr as usize) + (i + j)) as *const u8);
			let c = if v < 32 {
				'.'
			} else {
				v as char
			};
			::println!("{}", c);
			j += 1;
		}

		::println!("|");

		i += j;
	}
}

/*
 * Returns the name of the symbol at offset `offset`.
 */
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

/*
 * Returns an Option containing the name of the function for the given instruction pointer. If the
 * name cannot be retrieved, the function returns None.
 */
fn get_function_name(inst: *const Void) -> Option<&'static str> {
	if inst < memory::get_kernel_virtual_begin() || inst >= memory::get_kernel_virtual_end() {
		return None;
	}

	let boot_info = multiboot::get_boot_info();
	let mut func_name: Option<&'static str> = None;
	elf::foreach_sections(boot_info.elf_sections, boot_info.elf_num as usize, boot_info.elf_shndx as usize,
		boot_info.elf_entsize as usize, |hdr: &elf::ELF32SectionHeader, _name: &str| {
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

				if (inst as usize) >= value/* && (inst as usize) < (value + size)*/ { // TODO Fix overflow
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

/*
 * Prints the callstack in the current context, including symbol's name and address. `ebp` is value
 * of the `%ebp` register that is used as a starting point for printing. `max_depth` is the maximum
 * depth of the stack to print. If the stack is larger than the maximum depth, the function shall
 * print `...` at the end. If the callstack is empty, the function just prints `Empty`.
 */
pub fn print_callstack(ebp: *const u32, max_depth: usize) {
	::println!("--- Callstack ---");

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
			::println!("{}: {:p} -> {}", i, eip, name);
		} else {
			::println!("{}: {:p} -> ???", i, eip);
		}

		unsafe {
			ebp_ = *(ebp_ as *const u32) as *const u32;
		}
		i += 1;
	}

	if i == 0 {
		::println!("Empty");
	} else if ebp_ != (0 as *const _) {
		::println!("...");
	}
}
