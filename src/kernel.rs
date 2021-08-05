//! Maestro is a Unix kernel written in Rust. This reference documents interfaces for modules and
//! the kernel's internals.

#![no_std]
#![no_main]

#![feature(allow_internal_unstable)]
#![feature(asm)]
#![feature(coerce_unsized)]
#![feature(const_fn_trait_bound)]
#![feature(const_maybe_uninit_assume_init)]
#![feature(const_mut_refs)]
#![feature(const_ptr_offset)]
#![feature(const_raw_ptr_deref)]
#![feature(core_intrinsics)]
#![feature(custom_test_frameworks)]
#![feature(dispatch_from_dyn)]
#![feature(fundamental)]
#![feature(lang_items)]
#![feature(llvm_asm)]
#![feature(maybe_uninit_extra)]
#![feature(panic_info_message)]
#![feature(slice_ptr_get)]
#![feature(stmt_expr_attributes)]
#![feature(unsize)]

#![deny(warnings)]
#![allow(dead_code)]
#![allow(unused_macros)]

#![test_runner(crate::selftest::runner)]
#![reexport_test_harness_main = "kernel_selftest"]

// Importing all the code from the ABI library
// From the moment a library exist, Cargo runs the build script only for the library. Thus the
// binary must use the library to access C/asm symbols
use abi::*;

use core::ffi::c_void;
use crate::file::path::Path;
use crate::process::Process;

extern "C" {
	fn test_process();
}

/// This is the main function of the Rust source code, responsible for the initialization of the
/// kernel. When calling this function, the CPU must be in Protected Mode with the GDT loaded with
/// space for the Task State Segment.
/// `magic` is the magic number passed by Multiboot.
/// `multiboot_ptr` is the pointer to the Multiboot booting informations structure.
#[no_mangle]
pub extern "C" fn kernel_main(magic: u32, multiboot_ptr: *const c_void) -> ! {
	crate::cli!();
	tty::init();

	if magic != multiboot::BOOTLOADER_MAGIC || !util::is_aligned(multiboot_ptr, 8) {
		kernel_panic!("Bootloader non compliant with Multiboot2!", 0);
	}

	idt::init();
	pit::init();
	event::init();

	// TODO CPUID
	multiboot::read_tags(multiboot_ptr);

	memory::memmap::init(multiboot_ptr);
	if cfg!(config_debug_debug) {
		memory::memmap::print_entries();
	}
	memory::alloc::init();
	memory::malloc::init();
	let kernel_vmem = memory::vmem::kernel();
	if kernel_vmem.is_err() {
		crate::kernel_panic!("Cannot initialize kernel virtual memory!", 0);
	}

	#[cfg(test)]
	#[cfg(config_debug_test)]
	kernel_selftest();

	let args_parser = cmdline::ArgsParser::parse(&multiboot::get_boot_info().cmdline);
	if let Err(e) = args_parser {
		e.print();
		kern::halt();
	}
	let args_parser = args_parser.unwrap();
	logger::init(args_parser.is_silent());

	println!("Booting Maestro kernel version {}", kern::VERSION);

	println!("Initializing ACPI...");
	acpi::init();

	println!("Initializing ramdisks...");
	if device::storage::ramdisk::create().is_err() {
		kernel_panic!("Failed to create ramdisks!");
	}
	println!("Initializing devices management...");
	if device::init().is_err() {
		crate::kernel_panic!("Failed to initialize devices management!", 0);
	}

	let (root_major, root_minor) = args_parser.get_root_dev();
	println!("Root device is {} {}", root_major, root_minor);
	println!("Initializing files management...");
	if file::init(device::DeviceType::Block, root_major, root_minor).is_err() {
		kernel_panic!("Failed to initialize files management!");
	}
	if device::default::create().is_err() {
		kernel_panic!("Failed to create default devices!");
	}

	println!("Initializing processes...");
	if process::init().is_err() {
		kernel_panic!("Failed to init processes!", 0);
	}

	// TODO Start first process from disk (init program)
	let test_begin = unsafe {
		core::mem::transmute::<unsafe extern "C" fn(), *const c_void>(test_process)
	};
	if Process::new(None, 0, 0, test_begin, Path::root()).is_err() {
		kernel_panic!("Failed to create init process!", 0);
	}

	kern::enter_loop();
}

/// Function that is required to be implemented by the Rust compiler and is used only when
/// panicking.
#[lang = "eh_personality"]
fn eh_personality() {}
