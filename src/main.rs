/*
 * This file is the main source file for the kernel, but it is not the entry point of the kernel.
 */

#![no_std]
#![no_main]

#![feature(allow_internal_unstable)]
#![feature(asm)]
#![feature(const_in_array_repeat_expressions)]
#![feature(const_ptr_offset)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(custom_test_frameworks)]
#![feature(lang_items)]
#![feature(maybe_uninit_ref)]

#![deny(warnings)]
#![allow(dead_code)]
#![allow(unused_macros)]

/*
 * The following attributes allow to specify the location of the custom test framework for embedded
 * self-testing and debugging.
 */
#![test_runner(crate::selftest::runner)]
#![reexport_test_harness_main = "test_main"]

mod debug;
mod elf;
mod error;
#[macro_use]
mod idt;
mod memory;
mod multiboot;
#[macro_use]
mod panic;
mod pit;
#[macro_use]
mod print;
mod selftest;
mod tty;
#[macro_use]
mod util;
mod vga;

use core::ffi::c_void;
use core::panic::PanicInfo;

/*
 * Current kernel version.
 */
const KERNEL_VERSION: &'static str = "1.0";

extern "C" {
	pub fn kernel_wait();
	pub fn kernel_loop() -> !;
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

/*
 * This is the main function of the Rust source code, responsible for the initialization of the
 * kernel. When calling this function, the CPU must be in Protected Mode with the GDT loaded with
 * space for the Task State Segment.
 * `magic` is the magic number passed by Multiboot.
 * `multiboot_ptr` is the pointer to the Multiboot booting informations structure.
 */
#[no_mangle]
pub extern "C" fn kernel_main(magic: u32, multiboot_ptr: *const c_void) -> ! {
	tty::init();

	if magic != multiboot::BOOTLOADER_MAGIC || !util::is_aligned(multiboot_ptr, 8) {
		kernel_panic!("Bootloader non compliant with Multiboot2!", 0);
	}

	idt::init();
	pit::init();

	println!("Booting Maestro kernel version {}", KERNEL_VERSION);
	// TODO CPUID
	multiboot::read_tags(multiboot_ptr);

	println!("Initializing memory allocation...");
	memory::memmap::init(multiboot_ptr);
	memory::memmap::print_entries(); // TODO rm
	memory::buddy::init();
	memory::vmem::kernel();

	#[cfg(test)]
	test_main();

	// TODO Register default error handlers

	// TODO ACPI
	// TODO PCI
	// TODO time
	// TODO drivers
	// TODO Disk
	// TODO Process

	// TODO Load init ramdisk
	// TODO Start first process

	unsafe {
		kernel_halt(); // TODO Replace with kernel_loop
	}
}

/*
 * Called on Rust panic.
 */
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
/*
 * Called on Rust panic during testing.
 */
#[cfg(test)]
#[panic_handler]
fn panic(panic_info: &PanicInfo) -> ! {
	println!("FAILED\n");
	println!("Error: {}\n", panic_info);
	unsafe {
		kernel_halt();
	}
}

/*
 * TODO doc
 */
#[lang = "eh_personality"]
fn eh_personality() {
	// TODO Do something?
}

