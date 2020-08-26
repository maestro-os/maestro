#![no_std]
#![no_main]

#![feature(rustc_private)]
#![feature(lang_items)]

#![deny(warnings)]

extern crate libc;
use core::panic::PanicInfo;

extern "C" {
    fn tty_init();
}

#[no_mangle]
pub extern "C" fn kernel_main(_magic: u32, _multiboot_ptr: *const libc::c_void) {
    unsafe {
        tty_init();
    }

    // TODO
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

