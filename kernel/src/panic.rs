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

//! This module implements kernel panics handling.
//!
//! A kernel panic occurs when an error is raised that the kernel cannot recover
//! from. This is an undesirable state which requires to reboot the host
//! machine.

#[cfg(config_debug_qemu)]
use crate::debug::qemu;
use crate::{
	arch::{
		core_id,
		x86::{cli, idt::IntFrame},
	},
	logger::LOGGER,
	memory::VirtAddr,
	power, println, register_get,
};
use core::{
	fmt,
	panic::{Location, PanicInfo},
};

fn panic_impl(msg: impl fmt::Display, loc: Option<&Location>, frame: Option<&IntFrame>) -> ! {
	cli();
	LOGGER.lock().silent = false;
	// Print panic
	println!("-- KERNEL PANIC! --");
	let cpu = core_id();
	if let Some(loc) = loc {
		println!("CPU: {cpu} Reason: {msg} Location: {loc}");
	} else {
		println!("CPU: {cpu} Reason: {msg}");
	}
	if let Some(frame) = frame {
		println!("{frame}");
		#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
		{
			let cr2 = VirtAddr(register_get!("cr2"));
			let cr3 = VirtAddr(register_get!("cr3"));
			println!("CR2: {cr2:?} CR3: {cr3:?}");
		}
	}
	// Print callstack
	#[cfg(debug_assertions)]
	{
		use crate::debug;
		use core::ptr;

		println!("Callstack:");
		#[cfg(target_arch = "x86")]
		let frame = register_get!("ebp");
		#[cfg(target_arch = "x86_64")]
		let frame = register_get!("rbp");
		let frame = ptr::with_exposed_provenance(frame);
		const CALLSTACK_DEPTH: usize = build_cfg!(config_panic_callstack_depth);
		let mut callstack: [VirtAddr; CALLSTACK_DEPTH] = [VirtAddr::default(); CALLSTACK_DEPTH];
		unsafe {
			debug::get_callstack(frame, &mut callstack);
		}
		debug::print_callstack(&callstack);
	}
	println!("-- end trace --");
	#[cfg(config_debug_qemu)]
	qemu::exit(qemu::FAILURE);
	power::halt();
}

/// Called on Rust panic.
#[panic_handler]
fn panic(panic_info: &PanicInfo) -> ! {
	panic_impl(panic_info.message(), panic_info.location(), None);
}

/// The list of interrupt error messages ordered by index of the corresponding
/// interrupt vector.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
static INT_REASONS: &[&str] = &[
	"Divide-by-zero Error",
	"Debug",
	"Non-maskable Interrupt",
	"Breakpoint",
	"Overflow",
	"Bound Range Exceeded",
	"Invalid Opcode",
	"Device Not Available",
	"Double Fault",
	"Coprocessor Segment Overrun",
	"Invalid TSS",
	"Segment Not Present",
	"Stack-Segment Fault",
	"General Protection Fault",
	"Page Fault",
	"Unknown",
	"x87 Floating-Point Exception",
	"Alignment Check",
	"Machine Check",
	"SIMD Floating-Point Exception",
	"Virtualization Exception",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Security Exception",
	"Unknown",
];

/// Panics with the information of an interrupt frame.
pub fn with_frame(frame: &IntFrame) -> ! {
	let error = INT_REASONS.get(frame.int as usize).unwrap_or(&"Unknown");
	panic_impl(error, None, Some(frame));
}

// TODO check whether this can be removed since the kernel uses panic=abort
#[lang = "eh_personality"]
fn eh_personality() {}
