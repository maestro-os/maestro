#![no_std]
#![no_main]

#![feature(rustc_private)]
#![feature(lang_items)]

#![deny(warnings)]

use core::panic::PanicInfo;

extern "C" {
    fn kernel_main_(magic: u32, multiboot_ptr: *const u8);
}

#[no_mangle]
pub extern "C" fn kernel_main(magic: u32, multiboot_ptr: *const u8) {
    unsafe {
        kernel_main_(magic, multiboot_ptr);
    }
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

