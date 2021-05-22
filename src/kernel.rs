//! Maestro is a Unix kernel written in Rust. This reference documents interfaces for modules and
//! the kernel's internals.

#![no_std]
#![no_main]

#![feature(allow_internal_unstable)]
#![feature(asm)]
#![feature(coerce_unsized)]
#![feature(const_fn)]
#![feature(const_fn_trait_bound)]
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
mod cmdline;
mod debug;
mod device;
mod elf;
mod errno;
mod event;
mod file;
mod gdt;
#[macro_use]
mod idt;
mod limits;
mod logger;
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
use crate::file::path::Path;
use crate::process::Process;

/// Current kernel version.
const KERNEL_VERSION: &'static str = "1.0";

mod kern {
	use core::ffi::c_void;

	extern "C" {
		pub fn kernel_wait();
		pub fn kernel_loop() -> !;
		pub fn kernel_loop_reset(stack: *mut c_void) -> !;
		pub fn kernel_halt() -> !;
	}
}

/// Makes the kernel wait for an interrupt, then returns.
/// This function enables interrupts.
pub fn wait() {
	unsafe {
		kern::kernel_wait();
	}
}

/// Enters the kernel loop and processes every interrupts indefinitely.
pub fn enter_loop() -> ! {
	unsafe {
		kern::kernel_loop();
	}
}

/// Resets the stack to the given value, then calls `enter_loop`.
/// The function is unsafe because the pointer passed in parameter might be invalid.
pub unsafe fn loop_reset(stack: *mut c_void) -> ! {
	kern::kernel_loop_reset(stack);
}

/// Halts the kernel until reboot.
pub fn halt() -> ! {
	unsafe {
		kern::kernel_halt();
	}
}

mod io {
	extern "C" {
		/// Inputs a byte from the specified port.
		pub fn inb(port: u16) -> u8;
		/// Inputs a word from the specified port.
		pub fn inw(port: u16) -> u16;
		/// Inputs a long from the specified port.
		pub fn inl(port: u16) -> u32;
		/// Outputs a byte to the specified port.
		pub fn outb(port: u16, value: u8);
		/// Outputs a word to the specified port.
		pub fn outw(port: u16, value: u16);
		/// Outputs a long to the specified port.
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
		halt();
	}
	let args_parser = args_parser.unwrap();
	logger::set_silent(args_parser.is_silent());

	println!("Booting Maestro kernel version {}", KERNEL_VERSION);

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
	if let Ok(mut p) = Process::new(None, 0, test_begin, Path::root()) {
		println!("Test process PID: {}", p.lock().get().get_pid());
	} else {
		kernel_panic!("Failed to create test process!", 0);
	}

	enter_loop();
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
	halt();
}

/// Function that is required to be implemented by the Rust compiler and is used only when
/// panicking.
#[lang = "eh_personality"]
fn eh_personality() {}
