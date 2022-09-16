//! This module implements debugging tools.

use core::fmt;
use crate::elf;
use crate::memory;
use crate::multiboot;
use core::ffi::c_void;
use core::mem::size_of;
use core::str;

/// Prints, in hexadecimal, the content of the memory at the given location `ptr`, with the given
/// size `n` in bytes.
///
/// # Safety
/// The range of memory of size `n` starting at pointer `ptr` must be readable. If not, the
/// behaviour is undefined.
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

/// Prints a callstack, including symbols' names and addresses.
///
/// `ebp` is the value of the `%ebp` register that is used as a starting point for printing.
///
/// `max_depth` is the maximum depth of the stack to print. If the stack is larger than the maximum
/// depth, the function shall print `...` at the end.
///
/// `f`: The given closure is called for each print to be performed. If None, the
/// function uses the `print` macro instead.
///
/// If the callstack is empty, the function just prints `Empty`.
pub fn print_callstack<F: FnMut(fmt::Arguments)>(ebp: *const u32, max_depth: usize, mut f: F) {
	let boot_info = multiboot::get_boot_info();

	let mut i: usize = 0;
	let mut ebp_ = ebp;
	while !ebp_.is_null() && i < max_depth {
		let eip = unsafe { *((ebp_ as usize + size_of::<usize>()) as *const u32) as *const c_void };
		if eip < memory::PROCESS_END {
			break;
		}

		if let Some(name) = elf::get_function_name(
			memory::kern_to_virt(boot_info.elf_sections),
			boot_info.elf_num as usize,
			boot_info.elf_shndx as usize,
			boot_info.elf_entsize as usize,
			eip,
		) {
			let name = str::from_utf8(name).unwrap_or("<Invalid UTF8>");
			f(format_args!("{}: {:p} -> {}\n", i, eip, name))
		} else {
			f(format_args!("{}: {:p} -> ???\n", i, eip))
		}

		unsafe {
			ebp_ = *(ebp_ as *const u32) as *const u32;
		}
		i += 1;
	}

	if i == 0 {
		f(format_args!("Empty\n"));
	} else if !ebp_.is_null() {
		f(format_args!("...\n"));
	}
}
