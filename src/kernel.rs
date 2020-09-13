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

#![deny(warnings)]
#![allow(dead_code)]
#![allow(unused_macros)]

#[macro_use] mod util;

#[macro_use] mod debug;
#[macro_use] mod idt;
#[macro_use] mod memory;
#[macro_use] mod panic;
#[macro_use] mod tty;
#[macro_use] mod vga;

use core::panic::PanicInfo;

use memory::Void;

// TODO rm
extern "C" {
    fn kernel_main_(magic: u32, multiboot_ptr: *const u8);
}

extern "C" {
	pub fn kernel_wait() -> !;
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
pub extern "C" fn kernel_main(_magic: u32, _multiboot_ptr: *const Void) {
	tty::init();
	println!("Hello world!");

	/*if(magic != MULTIBOOT2_BOOTLOADER_MAGIC || !is_aligned(multiboot_ptr, 8))
		PANIC("Non Multiboot2-compliant bootloader!", 0);*/

	// TODO IDT init
	// TODO PIT init

	// TODO CPUID
	// TODO read boot tags

	// TODO memmap_init
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

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
	panic::kernel_panic("Rust panic: panic", 0);
}

#[lang = "eh_personality"]
fn eh_personality() {
	panic::kernel_panic("Rust panic: eh_personality", 0);
}

