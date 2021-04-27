/*
 * This file is the main source file for the kernel, but it is not the entry point of the kernel.
 */

#![no_std]
#![no_main]

#![feature(allow_internal_unstable)]
#![feature(asm)]
#![feature(coerce_unsized)]
#![feature(const_fn)]
#![feature(const_maybe_uninit_assume_init)]
#![feature(const_mut_refs)]
#![feature(const_ptr_offset)]
#![feature(const_raw_ptr_deref)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(core_intrinsics)]
#![feature(custom_test_frameworks)]
#![feature(dispatch_from_dyn)]
#![feature(fundamental)]
#![feature(lang_items)]
#![feature(llvm_asm)]
#![feature(maybe_uninit_extra)]
#![feature(maybe_uninit_ref)]
#![feature(panic_info_message)]
#![feature(slice_ptr_get)]
#![feature(stmt_expr_attributes)]
#![feature(unsize)]

#![deny(warnings)]
#![allow(dead_code)]
#![allow(unused_macros)]

#![test_runner(crate::selftest::runner)]
#![reexport_test_harness_main = "kernel_selftest"]

mod acpi;
mod debug;
mod device;
mod elf;
mod errno;
mod event;
mod filesystem;
mod gdt;
#[macro_use]
mod idt;
mod limits;
mod memory;
mod module;
mod multiboot;
#[macro_use]
mod panic;
mod pit;
#[macro_use]
mod print;
mod process;
mod selftest;
mod syscall;
mod time;
mod tty;
#[macro_use]
mod util;
#[macro_use]
mod vga;

use core::ffi::c_void;
use core::panic::PanicInfo;
use crate::filesystem::path::Path;
use crate::process::Process;

/// Current kernel version.
const KERNEL_VERSION: &'static str = "1.0";

extern "C" {
	pub fn kernel_wait();
	pub fn kernel_loop() -> !;
	pub fn kernel_loop_reset(stack: *mut c_void) -> !;
	pub fn kernel_halt() -> !;
}

mod io {
	extern "C" {
		pub fn inb(port: u16) -> u8;
		pub fn inw(port: u16) -> u16;
		pub fn inl(port: u16) -> u32;
		pub fn outb(port: u16, value: u8);
		pub fn outw(port: u16, value: u16);
		pub fn outl(port: u16, value: u32);
	}
}

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
	tty::init();

	if magic != multiboot::BOOTLOADER_MAGIC || !util::is_aligned(multiboot_ptr, 8) {
		kernel_panic!("Bootloader non compliant with Multiboot2!", 0);
	}

	idt::init();
	pit::init();
	event::init();

	println!("Booting Maestro kernel version {}", KERNEL_VERSION);
	// TODO CPUID
	multiboot::read_tags(multiboot_ptr);

	println!("Reading memory map...");
	memory::memmap::init(multiboot_ptr);
	if cfg!(config_debug_debug) {
		memory::memmap::print_entries();
	}

	println!("Initializing memory allocation...");
	memory::alloc::init();
	memory::malloc::init();

	println!("Initializing virtual memory handler...");
	let kernel_vmem = memory::vmem::kernel();
	if kernel_vmem.is_err() {
		crate::kernel_panic!("Cannot initialize kernel virtual memory!", 0);
	}

	#[cfg(test)]
	#[cfg(config_debug_test)]
	kernel_selftest();

	acpi::init();

	if device::default::create().is_err() {
		kernel_panic!("Failed to create default devices!");
	}
	if device::detect().is_err() {
		crate::kernel_panic!("Failed to detect device buses!", 0);
	}
	#[cfg(config_debug_storagetest)]
	storage::test();

	// TODO Load module through userspace instead
	println!("Loading modules...");
	// TODO Load modules from file and register into a vector

	// TODO Disk
	// TODO Virtual file system

	println!("Initializing processes...");
	if process::init().is_err() {
		kernel_panic!("Failed to init processes!", 0);
	}

	// TODO Load init ramdisk

	// TODO Start first process from disk (init program)
	let test_begin = unsafe { // Use of transmute
		core::mem::transmute::<unsafe extern "C" fn(), *const c_void>(test_process)
	};
	if let Ok(mut p) = Process::new(None, 0, test_begin, Path::root()) {
		println!("Test process PID: {}", p.lock().get().get_pid());
	} else {
		kernel_panic!("Failed to create test process!", 0);
	}

	unsafe {
		kernel_loop();
	}
}

/// Called on Rust panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(panic_info: &PanicInfo) -> ! {
	if let Some(s) = panic_info.message() {
		panic::rust_panic(s);
	} else {
		kernel_panic!("Rust panic (no payload)", 0);
	}
}

// TODO Use only if test was running. Else, use classic function
/// Called on Rust panic during testing.
#[cfg(test)]
#[panic_handler]
fn panic(panic_info: &PanicInfo) -> ! {
	println!("FAILED\n");
	println!("Error: {}\n", panic_info);
	unsafe {
		kernel_halt();
	}
}

/// TODO doc
#[lang = "eh_personality"]
fn eh_personality() {
	// TODO Do something?
}
