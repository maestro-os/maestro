#![no_std]
#![no_main]

#![feature(const_fn)]
#![feature(const_in_array_repeat_expressions)]
#![feature(const_ptr_offset)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(intrinsics)]
#![feature(lang_items)]
#![feature(rustc_attrs)]
#![feature(rustc_private)]

#![deny(warnings)]
#![allow(dead_code)]
#![allow(unused_macros)]

mod memory;
mod tty;
mod util;
mod vga;

use core::panic::PanicInfo;

extern "C" {
    fn kernel_main_(magic: u32, multiboot_ptr: *const u8);

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
pub extern "C" fn kernel_main(_magic: u32, _multiboot_ptr: *const u8) {
	vga::putchar('A', 0, 0);

    /*unsafe {
        kernel_main_(magic, multiboot_ptr);
    }*/
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // TODO Call kernel_panic
    loop {}
}

#[lang = "eh_personality"]
fn eh_personality() {
    // TODO Call kernel_panic
    loop {}
}

