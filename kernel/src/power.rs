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

//! This module handles system power.

use crate::{
	arch::x86::{
		apic,
		apic::{IpiDeliveryMode, lapic_id},
		cli, hlt,
		io::{inb, outb},
	},
	println,
	process::scheduler::{CPU, defer, per_cpu},
};
use core::{
	arch::asm,
	sync::atomic::{
		AtomicUsize,
		Ordering::{Acquire, Release},
	},
};

/// The number of halted cores.
///
/// When this value is greater than zero, all other CPU cores should halt and increment this
/// counter.
static HALTED_CORES: AtomicUsize = AtomicUsize::new(0);

/// Tells whether the system is currently halting.
#[inline]
pub fn halting() -> bool {
	HALTED_CORES.load(Acquire) > 0
}

fn notify_halt(log: &str) {
	let old = HALTED_CORES.fetch_add(1, Release);
	// If another CPU is notifying everyone, stop here
	if old > 0 {
		return;
	}
	println!("{log}");
	// Send IPI to other cores to halt them too
	if apic::is_present() {
		let lapic = lapic_id();
		CPU.iter()
			// Exclude current and offline cores
			.filter(|cpu| cpu.apic_id != lapic && cpu.online.load(Acquire))
			// Non-maskable interrupt
			.for_each(|cpu| apic::ipi(cpu.apic_id, IpiDeliveryMode::Nmi, defer::INT));
	}
	// Mark the current CPU as offline
	per_cpu().online.store(false, Release);
}

/// Halts the kernel until reboot.
pub fn halt() -> ! {
	cli();
	notify_halt("Halting...");
	loop {
		cli();
		hlt();
	}
}

/// Powers the system down.
pub fn shutdown() -> ! {
	cli();
	notify_halt("Power down...");
	todo!() // use ACPI to power off the system
}

/// Reboots the system.
pub fn reboot() -> ! {
	cli();
	notify_halt("Rebooting...");
	// First try: ACPI
	// TODO Use ACPI reset
	// Second try: PS/2
	loop {
		let tmp = unsafe { inb(0x64) };
		// Empty keyboard buffer
		if tmp & 0b1 != 0 {
			unsafe {
				inb(0x60);
			}
		}
		// If buffer is empty, break
		if tmp & 0b10 == 0 {
			break;
		}
	}
	// PS/2 CPU reset command
	unsafe {
		outb(0x64, 0xfe);
	}
	// Third try: triple fault
	unsafe {
		asm!("push 0xffff", "push 0", "retf");
	}
	unreachable!();
}
