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

//! The IDT (Interrupt Descriptor Table) is a table under the x86 architecture
//! storing the list of interrupt handlers, allowing to catch and handle
//! interruptions.

pub mod pic;

use core::{arch::asm, ffi::c_void, mem::size_of, ptr::addr_of};
use utils::interrupt::{cli, is_interrupt_enabled, sti};

/// Makes the interrupt switch to ring 0.
const ID_PRIVILEGE_RING_0: u8 = 0b00000000;
/// Makes the interrupt switch to ring 1.
const ID_PRIVILEGE_RING_1: u8 = 0b00000010;
/// Makes the interrupt switch to ring 2.
const ID_PRIVILEGE_RING_2: u8 = 0b00000100;
/// Makes the interrupt switch to ring 3.
const ID_PRIVILEGE_RING_3: u8 = 0b00000110;
/// Flag telling that the interrupt is present.
const ID_PRESENT: u8 = 0b00000001;

/// The IDT vector index for system calls.
pub const SYSCALL_ENTRY: usize = 0x80;
/// The number of entries into the IDT.
pub const ENTRIES_COUNT: usize = 0x81;

/// An IDT header.
#[repr(C, packed)]
struct InterruptDescriptorTable {
	/// The size of the IDT in bytes, minus 1.
	size: u16,
	/// The pointer to the beginning of the IDT.
	offset: u32,
}

/// An IDT entry.
#[repr(C, packed)]
#[derive(Clone, Copy)]
struct InterruptDescriptor {
	/// Bits 0..15 of the address to the handler for the interrupt.
	offset: u16,
	/// The code segment selector to execute the interrupt.
	selector: u16,
	/// Must be set to zero.
	zero: u8,
	/// Interrupt handler flags.
	flags: u8,
	/// Bits 16..31 of the address to the handler for the interrupt.
	offset_2: u16,
}

impl InterruptDescriptor {
	/// Returns a placeholder entry.
	///
	/// This function is necessary because the `const_trait_impl` feature is currently unstable,
	/// preventing to use `Default`.
	const fn placeholder() -> Self {
		Self {
			offset: 0,
			selector: 0,
			zero: 0,
			flags: 0,
			offset_2: 0,
		}
	}

	/// Creates an IDT entry.
	///
	/// Arguments:
	/// - `address` is the address of the handler.
	/// - `selector` is the segment selector to be used to handle the interrupt.
	/// - `flags` is the set of flags for the entry (see Intel documentation).
	fn new(address: *const c_void, selector: u16, flags: u8) -> Self {
		Self {
			offset: ((address as u32) & 0xffff) as u16,
			selector,
			zero: 0,
			flags,
			offset_2: (((address as u32) & 0xffff0000) >> utils::bit_size_of::<u16>()) as u16,
		}
	}
}

extern "C" {
	fn irq0();
	fn irq1();
	fn irq2();
	fn irq3();
	fn irq4();
	fn irq5();
	fn irq6();
	fn irq7();
	fn irq8();
	fn irq9();
	fn irq10();
	fn irq11();
	fn irq12();
	fn irq13();
	fn irq14();
	fn irq15();

	fn error0();
	fn error1();
	fn error2();
	fn error3();
	fn error4();
	fn error5();
	fn error6();
	fn error7();
	fn error8();
	fn error9();
	fn error10();
	fn error11();
	fn error12();
	fn error13();
	fn error14();
	fn error15();
	fn error16();
	fn error17();
	fn error18();
	fn error19();
	fn error20();
	fn error21();
	fn error22();
	fn error23();
	fn error24();
	fn error25();
	fn error26();
	fn error27();
	fn error28();
	fn error29();
	fn error30();
	fn error31();

	fn syscall();
}

/// The list of IDT entries.
static mut IDT_ENTRIES: [InterruptDescriptor; ENTRIES_COUNT] =
	[InterruptDescriptor::placeholder(); ENTRIES_COUNT];

/// Loads the given Interrupt Descriptor Table.
unsafe fn idt_load(idt: *const InterruptDescriptorTable) {
	asm!("lidt [{idt}]", idt = in(reg) idt);
}

/// Executes the given function `f` with maskable interruptions disabled.
///
/// This function saves the state of the interrupt flag and restores it before
/// returning.
pub fn wrap_disable_interrupts<T, F: FnOnce() -> T>(f: F) -> T {
	let int = is_interrupt_enabled();

	// Here is assumed that no interruption will change eflags. Which could cause a
	// race condition

	cli();

	let result = f();

	if int {
		sti();
	} else {
		cli();
	}

	result
}

/// Initializes the IDT.
///
/// This function must be called only once at kernel initialization.
///
/// When returning, maskable interrupts are disabled by default.
pub(crate) fn init() {
	cli();
	pic::init(0x20, 0x28);

	// Fill entries table
	let mut entries: [InterruptDescriptor; ENTRIES_COUNT] =
		[InterruptDescriptor::placeholder(); ENTRIES_COUNT];
	// Errors
	entries[0x00] = InterruptDescriptor::new(error0 as _, 0x8, 0x8e);
	entries[0x01] = InterruptDescriptor::new(error1 as _, 0x8, 0x8e);
	entries[0x02] = InterruptDescriptor::new(error2 as _, 0x8, 0x8e);
	entries[0x03] = InterruptDescriptor::new(error3 as _, 0x8, 0x8e);
	entries[0x04] = InterruptDescriptor::new(error4 as _, 0x8, 0x8e);
	entries[0x05] = InterruptDescriptor::new(error5 as _, 0x8, 0x8e);
	entries[0x06] = InterruptDescriptor::new(error6 as _, 0x8, 0x8e);
	entries[0x07] = InterruptDescriptor::new(error7 as _, 0x8, 0x8e);
	entries[0x08] = InterruptDescriptor::new(error8 as _, 0x8, 0x8e);
	entries[0x09] = InterruptDescriptor::new(error9 as _, 0x8, 0x8e);
	entries[0x0a] = InterruptDescriptor::new(error10 as _, 0x8, 0x8e);
	entries[0x0b] = InterruptDescriptor::new(error11 as _, 0x8, 0x8e);
	entries[0x0c] = InterruptDescriptor::new(error12 as _, 0x8, 0x8e);
	entries[0x0d] = InterruptDescriptor::new(error13 as _, 0x8, 0x8e);
	entries[0x0e] = InterruptDescriptor::new(error14 as _, 0x8, 0x8e);
	entries[0x0f] = InterruptDescriptor::new(error15 as _, 0x8, 0x8e);
	entries[0x10] = InterruptDescriptor::new(error16 as _, 0x8, 0x8e);
	entries[0x11] = InterruptDescriptor::new(error17 as _, 0x8, 0x8e);
	entries[0x12] = InterruptDescriptor::new(error18 as _, 0x8, 0x8e);
	entries[0x13] = InterruptDescriptor::new(error19 as _, 0x8, 0x8e);
	entries[0x14] = InterruptDescriptor::new(error20 as _, 0x8, 0x8e);
	entries[0x15] = InterruptDescriptor::new(error21 as _, 0x8, 0x8e);
	entries[0x16] = InterruptDescriptor::new(error22 as _, 0x8, 0x8e);
	entries[0x17] = InterruptDescriptor::new(error23 as _, 0x8, 0x8e);
	entries[0x18] = InterruptDescriptor::new(error24 as _, 0x8, 0x8e);
	entries[0x19] = InterruptDescriptor::new(error25 as _, 0x8, 0x8e);
	entries[0x1a] = InterruptDescriptor::new(error26 as _, 0x8, 0x8e);
	entries[0x1b] = InterruptDescriptor::new(error27 as _, 0x8, 0x8e);
	entries[0x1c] = InterruptDescriptor::new(error28 as _, 0x8, 0x8e);
	entries[0x1d] = InterruptDescriptor::new(error29 as _, 0x8, 0x8e);
	entries[0x1e] = InterruptDescriptor::new(error30 as _, 0x8, 0x8e);
	entries[0x1f] = InterruptDescriptor::new(error31 as _, 0x8, 0x8e);
	// PIC interruptions
	entries[0x20] = InterruptDescriptor::new(irq0 as _, 0x8, 0x8e);
	entries[0x21] = InterruptDescriptor::new(irq1 as _, 0x8, 0x8e);
	entries[0x22] = InterruptDescriptor::new(irq2 as _, 0x8, 0x8e);
	entries[0x23] = InterruptDescriptor::new(irq3 as _, 0x8, 0x8e);
	entries[0x24] = InterruptDescriptor::new(irq4 as _, 0x8, 0x8e);
	entries[0x25] = InterruptDescriptor::new(irq5 as _, 0x8, 0x8e);
	entries[0x26] = InterruptDescriptor::new(irq6 as _, 0x8, 0x8e);
	entries[0x27] = InterruptDescriptor::new(irq7 as _, 0x8, 0x8e);
	entries[0x28] = InterruptDescriptor::new(irq8 as _, 0x8, 0x8e);
	entries[0x29] = InterruptDescriptor::new(irq9 as _, 0x8, 0x8e);
	entries[0x2a] = InterruptDescriptor::new(irq10 as _, 0x8, 0x8e);
	entries[0x2b] = InterruptDescriptor::new(irq11 as _, 0x8, 0x8e);
	entries[0x2c] = InterruptDescriptor::new(irq12 as _, 0x8, 0x8e);
	entries[0x2d] = InterruptDescriptor::new(irq13 as _, 0x8, 0x8e);
	entries[0x2e] = InterruptDescriptor::new(irq14 as _, 0x8, 0x8e);
	entries[0x2f] = InterruptDescriptor::new(irq15 as _, 0x8, 0x8e);
	// System calls
	entries[SYSCALL_ENTRY] = InterruptDescriptor::new(syscall as _, 0x8, 0xee);

	// Safe because the current function is called only once at boot
	unsafe {
		IDT_ENTRIES = entries;
	}
	let idt = InterruptDescriptorTable {
		size: (size_of::<InterruptDescriptor>() * ENTRIES_COUNT - 1) as u16,
		offset: unsafe { IDT_ENTRIES.as_ptr() } as _,
	};
	unsafe {
		idt_load(addr_of!(idt));
	}
}
