//! This module implements debugging tools.

use core::ffi::c_void;
use core::mem::size_of;
use core::ptr::null_mut;
use core::str;
use crate::elf;
use crate::memory;
use crate::multiboot;

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

/// Fills the slice `stack` with the callstack starting at `ebp`. The first element is the last
/// called function and the last element is the first called function.
/// When the stack ends, the function fills the rest of the slice with None.
pub fn get_callstack(ebp: *mut u32, stack: &mut [*mut c_void]) {
	stack.fill(null_mut::<c_void>());

	let mut i = 0;
	let mut frame = ebp;

	while !frame.is_null() && i < stack.len() {
		let pc = unsafe {
			*((frame as usize + size_of::<usize>()) as *mut u32) as *mut c_void
		};
		if pc < memory::PROCESS_END as *mut c_void {
			break;
		}

		stack[i] = pc;

		unsafe {
			frame = *(frame as *mut u32) as *mut u32;
		}
		i += 1;
	}
}

/// Prints a callstack, including symbols' names and addresses.
///
/// `stack` is the callstack to print.
///
/// If the callstack is empty, the function just prints `Empty`.
pub fn print_callstack(stack: &[*mut c_void]) {
	if stack.is_empty() || stack[0].is_null() {
		crate::println!("Empty");
		return;
	}

	let boot_info = multiboot::get_boot_info();
	for (i, pc) in stack.iter().enumerate() {
		if pc.is_null() {
			break;
		}

		let name_result = elf::get_function_name(
			memory::kern_to_virt(boot_info.elf_sections),
			boot_info.elf_num as usize,
			boot_info.elf_shndx as usize,
			boot_info.elf_entsize as usize,
			*pc,
		);

		match name_result {
			Some(name) => {
				let name = str::from_utf8(name).unwrap_or("<Invalid UTF8>");
				crate::println!("{}: {:p} -> {}", i, pc, name);
			},

			None => crate::println!("{}: {:p} -> ???", i, pc),
		}
	}
}
