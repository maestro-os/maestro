/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! Maestro is a Unix kernel written in Rust. This reference documents
//! interfaces for modules and the kernel's internals.
//!
//! # Features
//!
//! The crate has the following features:
//! - `strace`: if enabled, the kernel traces system calls. This is a debug feature.

#![no_std]
#![no_main]
#![feature(allocator_api)]
#![feature(allow_internal_unstable)]
#![feature(array_chunks)]
#![feature(asm_const)]
#![feature(associated_type_defaults)]
#![feature(coerce_unsized)]
#![feature(core_intrinsics)]
#![feature(custom_test_frameworks)]
#![feature(dispatch_from_dyn)]
#![feature(exclusive_range_pattern)]
#![feature(is_sorted)]
#![feature(iter_array_chunks)]
#![feature(iter_intersperse)]
#![feature(iterator_try_collect)]
#![feature(lang_items)]
#![feature(nonzero_ops)]
#![feature(offset_of)]
#![feature(once_cell_try)]
#![feature(panic_info_message)]
#![feature(pointer_is_aligned)]
#![feature(portable_simd)]
#![feature(ptr_metadata)]
#![feature(set_ptr_value)]
#![feature(slice_index_methods)]
#![feature(stmt_expr_attributes)]
#![feature(strict_provenance)]
#![feature(trusted_len)]
#![feature(unsize)]
#![deny(warnings)]
#![allow(clippy::tabs_in_doc_comments)]
#![allow(dead_code)]
#![allow(internal_features)]
#![allow(unused_attributes)]
#![allow(unused_macros)]
#![test_runner(crate::selftest::runner)]
#![reexport_test_harness_main = "kernel_selftest"]

extern crate alloc;

pub mod acpi;
pub mod cmdline;
pub mod cpu;
pub mod crypto;
pub mod debug;
pub mod device;
pub mod elf;
pub mod event;
pub mod file;
#[cfg(target_arch = "x86")]
pub mod gdt;
#[macro_use]
pub mod idt;
pub mod io;
pub mod limits;
pub mod logger;
pub mod memory;
pub mod module;
pub mod multiboot;
pub mod net;
#[macro_use]
pub mod panic;
pub mod power;
#[macro_use]
pub mod print;
pub mod process;
pub mod selftest;
pub mod syscall;
pub mod time;
pub mod tty;
#[macro_use]
pub use utils;

use crate::{
	file::{fs::initramfs, path::Path, vfs, vfs::ResolutionSettings},
	logger::LOGGER,
	memory::vmem,
	process::{exec, exec::ExecInfo, Process},
};
use core::{arch::asm, ffi::c_void};
use utils::{
	collections::{string::String, vec::Vec},
	errno::EResult,
	lock::Mutex,
	vec,
};

/// The kernel's name.
pub const NAME: &str = env!("CARGO_PKG_NAME");
/// Current kernel version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The name of the current architecture.
pub const ARCH: &str = "x86";

/// The path to the init process binary.
const INIT_PATH: &[u8] = b"/sbin/init";

/// The current hostname of the system.
pub static HOSTNAME: Mutex<Vec<u8>> = Mutex::new(Vec::new());

extern "C" {
	fn kernel_loop_reset(stack: *mut c_void) -> !;
}

/// Makes the kernel wait for an interrupt, then returns.
/// This function enables interruptions.
#[inline(always)]
pub fn wait() {
	unsafe {
		asm!("sti", "hlt");
	}
}

/// Enters the kernel loop and processes every interrupts indefinitely.
pub fn enter_loop() -> ! {
	loop {
		wait();
	}
}

/// Resets the stack to the given value, then calls [`enter_loop`].
///
/// # Safety
///
/// The callee must ensure the given stack is usable.
pub unsafe fn loop_reset(stack: *mut c_void) -> ! {
	kernel_loop_reset(stack);
}

/// Launches the init process.
///
/// `init_path` is the path to the init program.
fn init(init_path: String) -> EResult<()> {
	// The initial environment
	let env: Vec<String> = vec![
		b"PATH=/bin:/sbin:/usr/bin:/usr/sbin:/usr/local/bin:/usr/local/sbin".try_into()?,
		b"TERM=maestro".try_into()?,
	]?;

	let rs = ResolutionSettings::kernel_follow();

	let path = Path::new(&init_path)?;
	let file_mutex = vfs::get_file_from_path(path, &rs)?;
	let mut file = file_mutex.lock();

	let exec_info = ExecInfo {
		path_resolution: &rs,
		argv: vec![init_path]?,
		envp: env,
	};
	let program_image = exec::build_image(&mut file, exec_info)?;

	let proc_mutex = Process::new()?;
	let mut proc = proc_mutex.lock();
	exec::exec(&mut proc, program_image)
}

/// An inner function is required to ensure everything in scope is dropped before calling
/// [`enter_loop`].
fn kernel_main_inner(magic: u32, multiboot_ptr: *const c_void) {
	// Initialize TTY
	tty::init();
	// Ensure the CPU has SSE
	if !cpu::sse::is_present() {
		panic!("SSE support is required to run this kernel :(");
	}
	cpu::sse::enable();
	// Initialize IDT
	idt::init();

	// Read multiboot information
	if magic != multiboot::BOOTLOADER_MAGIC || !multiboot_ptr.is_aligned_to(8) {
		panic!("Bootloader non compliant with Multiboot2!");
	}
	unsafe {
		multiboot::read_tags(multiboot_ptr);
	}

	// Initialize memory management
	memory::memmap::init(multiboot_ptr);
	#[cfg(config_debug_debug)]
	memory::memmap::print_entries();
	memory::alloc::init();
	vmem::init()
		.unwrap_or_else(|_| panic!("Cannot initialize kernel virtual memory! (out of memory)"));

	// From now on, the kernel considers that memory management has been fully
	// initialized

	// Init kernel symbols map
	elf::kernel::init()
		.unwrap_or_else(|_| panic!("Cannot initialize kernel symbols map! (out of memory)"));

	// Perform kernel self-tests
	#[cfg(test)]
	kernel_selftest();

	let boot_info = multiboot::get_boot_info();

	// Parse bootloader command line arguments
	let cmdline = boot_info.cmdline.unwrap_or_default();
	let args_parser = match cmdline::ArgsParser::parse(cmdline) {
		Ok(p) => p,
		Err(e) => {
			println!("{e}");
			power::halt();
		}
	};
	LOGGER.lock().silent = args_parser.is_silent();

	println!("Booting Maestro kernel version {VERSION}");

	// FIXME
	//println!("Initializing ACPI...");
	//acpi::init();

	println!("Initializing time management...");
	time::init().unwrap_or_else(|e| panic!("Failed to initialize time management! ({e})"));

	// FIXME
	/*println!("Initializing ramdisks...");
	device::storage::ramdisk::create()
		.unwrap_or_else(|e| kernel_panic!("Failed to create ramdisks! ({})", e));*/
	println!("Initializing devices management...");
	device::init().unwrap_or_else(|e| panic!("Failed to initialize devices management! ({e})"));
	net::osi::init().unwrap_or_else(|e| panic!("Failed to initialize network! ({e})"));
	crypto::init()
		.unwrap_or_else(|_| panic!("Failed to initialize cryptography! (out of memory)"));

	let root = args_parser.get_root_dev();
	println!("Initializing files management...");
	file::init(root).unwrap_or_else(|e| panic!("Failed to initialize files management! ({e})"));
	if let Some(initramfs) = boot_info.initramfs {
		println!("Initializing initramfs...");
		initramfs::load(initramfs)
			.unwrap_or_else(|e| panic!("Failed to initialize initramfs! ({e})"));
	}
	device::stage2().unwrap_or_else(|e| panic!("Failed to create device files! ({e})"));

	println!("Initializing processes...");
	process::init().unwrap_or_else(|e| panic!("Failed to init processes! ({e})"));

	let init_path = args_parser.get_init_path().unwrap_or(INIT_PATH);
	let init_path = String::try_from(init_path).unwrap();
	init(init_path).unwrap_or_else(|e| panic!("Cannot execute init process: {e}"));
}

/// This is the main function of the Rust source code, responsible for the
/// initialization of the kernel.
///
/// When calling this function, the CPU must be in Protected Mode with the GDT loaded with space
/// for the Task State Segment.
///
/// Arguments:
/// - `magic` is the magic number passed by Multiboot.
/// - `multiboot_ptr` is the pointer to the Multiboot booting information
/// structure.
#[no_mangle]
pub extern "C" fn kernel_main(magic: u32, multiboot_ptr: *const c_void) -> ! {
	kernel_main_inner(magic, multiboot_ptr);
	enter_loop();
}
