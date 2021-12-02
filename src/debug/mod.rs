//! This module implements debugging tools.

use core::ffi::c_void;
use core::mem::size_of;
use core::str;
use crate::elf;
use crate::memory;
use crate::multiboot;

/// Prints, in hexadecimal, the content of the memory at the given location `ptr`, with the given
/// size `n` in bytes.
pub unsafe fn print_memory(ptr: *const c_void, n: usize) {
	let mut i = 0;

	while i < n {
		crate::print!("{:#08x}  ", ptr as usize + i);

		let mut j = 0;
		while j < 16 {
			if i + j < n {
				crate::print!("{:02x} ", *(((ptr as usize) + (i + j)) as *const u8));
			} else {
				crate::print!("   ");
			}

			j += 1;
		}

		crate::print!(" |");

		j = 0;
		while j < 16 && i + j < n {
			let val = *(((ptr as usize) + (i + j)) as *const u8);
			let c = {
				if (32..127).contains(&val) {
					val as char
				} else {
					'.'
				}
			};

			crate::print!("{}", c);
			j += 1;
		}

		crate::println!("|");

		i += j;
	}
}

/// Prints the callstack in the current context, including symbol's name and address. `ebp` is
/// value of the `%ebp` register that is used as a starting point for printing. `max_depth` is the
/// maximum depth of the stack to print. If the stack is larger than the maximum depth, the
/// function shall print `...` at the end. If the callstack is empty, the function just prints
/// `Empty`.
pub fn print_callstack(ebp: *const u32, max_depth: usize) {
	let boot_info = multiboot::get_boot_info();

	crate::println!("--- Callstack ---");

	let mut i: usize = 0;
	let mut ebp_ = ebp;
	while !ebp_.is_null() && i < max_depth {
		let eip = unsafe {
			*((ebp_ as usize + size_of::<usize>()) as *const u32) as *const c_void
		};
		if eip.is_null() {
			break;
		}

		if let Some(name) = elf::get_function_name(memory::kern_to_virt(boot_info.elf_sections),
			boot_info.elf_num as usize, boot_info.elf_shndx as usize,
			boot_info.elf_entsize as usize, eip) {

			let name = str::from_utf8(name).unwrap_or("<Invalid UTF8>");
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
	} else if !ebp_.is_null() {
		crate::println!("...");
	}
}
