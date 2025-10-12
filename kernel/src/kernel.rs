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
#![feature(adt_const_params)]
#![feature(alloc_layout_extra)]
#![feature(allocator_api)]
#![feature(allow_internal_unstable)]
#![feature(array_chunks)]
#![feature(custom_test_frameworks)]
#![feature(debug_closure_helpers)]
#![feature(lang_items)]
#![feature(likely_unlikely)]
#![feature(negative_impls)]
#![feature(non_null_from_ref)]
#![feature(offset_of_enum)]
#![feature(once_cell_try)]
#![feature(pointer_is_aligned_to)]
#![feature(ptr_metadata)]
#![feature(strict_provenance_lints)]
#![feature(unsigned_nonzero_div_ceil)]
#![deny(fuzzy_provenance_casts)]
#![deny(missing_docs)]
#![allow(clippy::tabs_in_doc_comments)]
#![allow(dead_code)]
#![allow(incomplete_features)]
#![allow(internal_features)]
#![allow(unsafe_op_in_unsafe_fn)]
#![test_runner(crate::selftest::runner)]
#![reexport_test_harness_main = "kernel_selftest"]

pub mod acpi;
pub mod arch;
mod boot;
pub mod cmdline;
#[macro_use]
pub mod config;
pub mod debug;
pub mod device;
pub mod elf;
pub mod file;
pub mod int;
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
pub mod rand;
pub mod selftest;
pub mod sync;
pub mod syscall;
pub mod time;
pub mod tty;

use crate::{
	arch::x86::{idt::IntFrame, smp},
	file::{fs::initramfs, vfs, vfs::ResolutionSettings},
	logger::LOGGER,
	memory::{cache, vmem},
	process::{
		Process, exec,
		exec::{ExecInfo, exec},
		scheduler,
		scheduler::{
			cpu::{CPU, per_cpu},
			switch,
			switch::idle_task,
		},
	},
	sync::mutex::Mutex,
	tty::TTY,
};
use core::ffi::c_void;
pub use utils;
use utils::{
	collections::{path::Path, string::String, vec::Vec},
	errno::EResult,
	vec,
};

/// The kernel's name.
pub const NAME: &str = env!("CARGO_PKG_NAME");
/// Current kernel version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The path to the init process binary.
const INIT_PATH: &[u8] = b"/sbin/init";

/// The current hostname of the system.
pub static HOSTNAME: Mutex<Vec<u8>> = Mutex::new(Vec::new());

/// Launches the init process.
///
/// `init_path` is the path to the init program.
///
/// On success, the function does not return.
fn init(init_path: String) -> EResult<IntFrame> {
	let mut frame = IntFrame::default();
	{
		let path = Path::new(&init_path)?;
		let rs = ResolutionSettings::kernel_follow();
		let ent = vfs::get_file_from_path(path, &rs)?;
		let program_image = exec::elf::exec(
			ent,
			ExecInfo {
				path_resolution: &rs,
				argv: vec![init_path]?,
				envp: vec![
					b"PATH=/bin:/sbin:/usr/bin:/usr/sbin:/usr/local/bin:/usr/local/sbin"
						.try_into()?,
					b"TERM=maestro".try_into()?,
				]?,
			},
		)?;
		let proc = Process::init()?;
		exec(&proc, &mut frame, program_image)?;
		per_cpu().sched.swap_current_process(proc);
	}
	Ok(frame)
}

/// An inner function is required to ensure everything in scope is dropped before idle.
fn kernel_main_inner(magic: u32, multiboot_ptr: *const c_void) {
	// Initialize TTY
	TTY.display.lock().show();
	// Architecture-specific initialization, stage 1
	arch::init1(true);

	println!("Boot {NAME} version {VERSION}");

	// Read multiboot information
	let boot_info = unsafe { multiboot::read(magic, multiboot_ptr) };

	// Initialize memory management
	println!("Setup memory management");
	memory::memmap::init(boot_info);
	memory::alloc::init();
	vmem::init();

	// From now on, the kernel considers that memory management has been fully
	// initialized

	// Init kernel symbols map
	elf::kernel::init().expect("cannot initialize kernel symbols map");

	// Perform kernel self-tests
	#[cfg(test)]
	kernel_selftest();

	// Parse bootloader command line arguments
	let cmdline = boot_info.cmdline.unwrap_or_default();
	let args_parser = cmdline::ArgsParser::parse(cmdline).expect("could not parse command line");
	LOGGER.lock().silent = args_parser.is_silent();

	println!("Find ACPI structures");
	acpi::init().expect("ACPI initialization failed");
	// Architecture-specific initialization, stage 2
	arch::init2(true).expect("architecture-specific initialization failed");

	println!("Setup time management");
	time::init().expect("time management initialization failed");

	println!("Setup devices management");
	device::init().expect("devices management initialization failed");
	net::osi::init().expect("network initialization failed");
	rand::init().expect("entropy pool initialization failed");

	let root = args_parser.get_root_dev();
	println!("Setup files management");
	file::init(root).expect("files management initialization failed");
	if let Some(initramfs) = boot_info.initramfs {
		println!("Load initramfs");
		initramfs::load(initramfs).expect("initramfs loading failed");
	}
	device::stage2().expect("device files creation failure");

	println!("Setup SMP");
	smp::init().expect("SMP setup failed");
	println!("Setup processes");
	process::init().expect("processes initialization failed");
	exec::vdso::init().expect("vDSO loading failed");

	let init_path = args_parser.get_init_path().unwrap_or(INIT_PATH);
	let init_path = String::try_from(init_path).unwrap();
	println!("Execute init process ({init_path})");
	let init_frame = init(init_path).expect("init process execution failed");

	// Launch kernel tasks
	if CPU.len() > 1 {
		Process::new_kthread(None, scheduler::rebalance_task, true)
			.expect("rebalance task launch failed");
	}
	Process::new_kthread(None, cache::flush_task, true).expect("cache flush task launch failed");

	unsafe {
		switch::init_ctx(&init_frame);
	}
}

/// This is the main function of the Rust source code, responsible for the
/// initialization of the kernel.
///
/// When calling this function, the CPU must be in Protected Mode with the GDT loaded with space
/// for the Task State Segment.
///
/// Arguments:
/// - `magic` is the magic number passed by Multiboot.
/// - `multiboot_ptr` is the pointer to the Multiboot booting information structure.
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(magic: u32, multiboot_ptr: *const c_void) -> ! {
	kernel_main_inner(magic, multiboot_ptr);
	unsafe {
		idle_task();
	}
}
