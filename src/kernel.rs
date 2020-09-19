#![no_std]
#![no_main]

#![feature(allow_internal_unstable)]
#![feature(asm)]
#![feature(const_fn)]
#![feature(const_in_array_repeat_expressions)]
#![feature(const_ptr_offset)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(intrinsics)]
#![feature(lang_items)]
#![feature(llvm_asm)]
#![feature(rustc_attrs)]
#![feature(rustc_private)]
#![feature(untagged_unions)]

#![deny(warnings)]
#![allow(dead_code)]
#![allow(unused_macros)]

mod debug;
mod idt;
mod memory;
mod multiboot;
mod panic;
mod pit;
mod tty;
mod util;
mod vga;

use core::panic::PanicInfo;

use memory::Void;

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

#[no_mangle]
pub extern "C" fn kernel_main(magic: u32, multiboot_ptr: *const Void) {
	tty::init();

	if magic != multiboot::BOOTLOADER_MAGIC || !util::is_aligned(multiboot_ptr, 8) {
		panic::kernel_panic("Bootloader non compliant with Multiboot2!", 0);
	}

	idt::init();
	pit::init();

	println!("Booting Maestro kernel version {}", KERNEL_VERSION);
	// TODO CPUID
	multiboot::read_tags(multiboot_ptr);

	println!("Initializing memory allocation...");
	memory::memmap::init(multiboot_ptr);
	// TODO buddy_init
	// TODO vmem_kernel

	// TODO ACPI
	// TODO PCI
	// TODO time
	// TODO drivers
	// TODO Disk
	// TODO Process

	unsafe {
		kernel_halt(); // TODO Replace with kernel_loop
	}
}

/*
 * Called on Rust panic.
 */
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
	panic::kernel_panic("Rust panic: panic", 0);
}

/*
 * TODO
 */
#[lang = "eh_personality"]
fn eh_personality() {
	panic::kernel_panic("Rust panic: eh_personality", 0);
}

