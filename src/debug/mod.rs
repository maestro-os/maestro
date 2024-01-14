//! Debugging tools for the kernel.

use crate::elf;
use crate::memory;
use crate::multiboot;
use crate::util::DisplayableStr;
use core::ffi::c_void;
use core::ptr::null_mut;

/// Fills the slice `stack` with the callstack starting at `frame`.
///
/// The first element is the last called function and the last element is the first called
/// function.
///
/// When the stack ends, the function fills the rest of the slice with `None`.
///
/// # Safety
///
/// The caller must ensure the `frame` parameter points ta a valid stack frame.
pub unsafe fn get_callstack(mut frame: *mut usize, stack: &mut [*mut c_void]) {
	stack.fill(null_mut::<c_void>());
	for f in stack.iter_mut() {
		if frame.is_null() {
			break;
		}

		let pc = (*frame.add(1)) as *mut c_void;
		if pc < memory::PROCESS_END {
			break;
		}
		*f = pc;
		frame = *frame as *mut usize;
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

		let name = elf::get_function_name(
			memory::kern_to_virt(boot_info.elf_sections),
			boot_info.elf_num as usize,
			boot_info.elf_shndx as usize,
			boot_info.elf_entsize as usize,
			*pc,
		)
		.unwrap_or(b"???");
		crate::println!("{i}: {pc:p} -> {}", DisplayableStr(name));
	}
}
