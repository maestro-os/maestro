//! The IDT (Interrupt Descriptor Table) is a table under the x86 architecture
//! storing the list of interrupt handlers, allowing to catch and handle
//! interruptions.

pub mod pic;

use crate::util;
use core::ffi::c_void;
use core::mem::size_of;
use core::mem::MaybeUninit;

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

/// Disables interruptions.
#[macro_export]
macro_rules! cli {
	() => {
		#[allow(unused_unsafe)]
		unsafe {
			core::arch::asm!("cli");
		}
	};
}

/// Enables interruptions.
#[macro_export]
macro_rules! sti {
	() => {
		#[allow(unused_unsafe)]
		unsafe {
			core::arch::asm!("sti");
		}
	};
}

/// Waits for an interruption.
#[macro_export]
macro_rules! hlt {
	() => {
		#[allow(unused_unsafe)]
		unsafe {
			core::arch::asm!("hlt");
		}
	};
}

/// Structure representing the IDT.
#[repr(C, packed)]
struct InterruptDescriptorTable {
	/// The size of the IDT in bytes, minus 1.
	size: u16,
	/// The pointer to the beginning of the IDT.
	offset: u32,
}

/// Structure representing an IDT entry.
#[repr(C)]
struct InterruptDescriptor {
	/// Bits 0..15 of the address to the handler for the interrupt.
	offset: u16,
	/// The code segment selector to execute the interrupt.
	selector: u16,
	/// Must be set to zero.
	zero: u8,
	/// Interrupt flags.
	type_attr: u8,
	/// Bits 16..31 of the address to the handler for the interrupt.
	offset_2: u16,
}

extern "C" {
	fn idt_load(idt: *const c_void);
	fn interrupt_is_enabled() -> i32;
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
static mut ID: MaybeUninit<[InterruptDescriptor; ENTRIES_COUNT]> = MaybeUninit::uninit();

/// Creates an IDT entry.
fn create_id(address: *const c_void, selector: u16, type_attr: u8) -> InterruptDescriptor {
	InterruptDescriptor {
		offset: ((address as u32) & 0xffff) as u16,
		selector,
		zero: 0,
		type_attr,
		offset_2: (((address as u32) & 0xffff0000) >> util::bit_size_of::<u16>()) as u16,
	}
}

/// Initializes the IDT.
///
/// This function must be called only once at kernel initialization.
///
/// When returning, maskable interrupts are disabled by default.
pub fn init() {
	cli!();
	pic::init(0x20, 0x28);

	// Access to global variable. Safe because the function is supposed to be called
	// only once
	unsafe {
		let id = ID.assume_init_mut();

		id[0x00] = create_id(error0 as _, 0x8, 0x8e);
		id[0x01] = create_id(error1 as _, 0x8, 0x8e);
		id[0x02] = create_id(error2 as _, 0x8, 0x8e);
		id[0x03] = create_id(error3 as _, 0x8, 0x8e);
		id[0x04] = create_id(error4 as _, 0x8, 0x8e);
		id[0x05] = create_id(error5 as _, 0x8, 0x8e);
		id[0x06] = create_id(error6 as _, 0x8, 0x8e);
		id[0x07] = create_id(error7 as _, 0x8, 0x8e);
		id[0x08] = create_id(error8 as _, 0x8, 0x8e);
		id[0x09] = create_id(error9 as _, 0x8, 0x8e);
		id[0x0a] = create_id(error10 as _, 0x8, 0x8e);
		id[0x0b] = create_id(error11 as _, 0x8, 0x8e);
		id[0x0c] = create_id(error12 as _, 0x8, 0x8e);
		id[0x0d] = create_id(error13 as _, 0x8, 0x8e);
		id[0x0e] = create_id(error14 as _, 0x8, 0x8e);
		id[0x0f] = create_id(error15 as _, 0x8, 0x8e);
		id[0x10] = create_id(error16 as _, 0x8, 0x8e);
		id[0x11] = create_id(error17 as _, 0x8, 0x8e);
		id[0x12] = create_id(error18 as _, 0x8, 0x8e);
		id[0x13] = create_id(error19 as _, 0x8, 0x8e);
		id[0x14] = create_id(error20 as _, 0x8, 0x8e);
		id[0x15] = create_id(error21 as _, 0x8, 0x8e);
		id[0x16] = create_id(error22 as _, 0x8, 0x8e);
		id[0x17] = create_id(error23 as _, 0x8, 0x8e);
		id[0x18] = create_id(error24 as _, 0x8, 0x8e);
		id[0x19] = create_id(error25 as _, 0x8, 0x8e);
		id[0x1a] = create_id(error26 as _, 0x8, 0x8e);
		id[0x1b] = create_id(error27 as _, 0x8, 0x8e);
		id[0x1c] = create_id(error28 as _, 0x8, 0x8e);
		id[0x1d] = create_id(error29 as _, 0x8, 0x8e);
		id[0x1e] = create_id(error30 as _, 0x8, 0x8e);
		id[0x1f] = create_id(error31 as _, 0x8, 0x8e);

		id[0x20] = create_id(irq0 as _, 0x8, 0x8e);
		id[0x21] = create_id(irq1 as _, 0x8, 0x8e);
		id[0x22] = create_id(irq2 as _, 0x8, 0x8e);
		id[0x23] = create_id(irq3 as _, 0x8, 0x8e);
		id[0x24] = create_id(irq4 as _, 0x8, 0x8e);
		id[0x25] = create_id(irq5 as _, 0x8, 0x8e);
		id[0x26] = create_id(irq6 as _, 0x8, 0x8e);
		id[0x27] = create_id(irq7 as _, 0x8, 0x8e);
		id[0x28] = create_id(irq8 as _, 0x8, 0x8e);
		id[0x29] = create_id(irq9 as _, 0x8, 0x8e);
		id[0x2a] = create_id(irq10 as _, 0x8, 0x8e);
		id[0x2b] = create_id(irq11 as _, 0x8, 0x8e);
		id[0x2c] = create_id(irq12 as _, 0x8, 0x8e);
		id[0x2d] = create_id(irq13 as _, 0x8, 0x8e);
		id[0x2e] = create_id(irq14 as _, 0x8, 0x8e);
		id[0x2f] = create_id(irq15 as _, 0x8, 0x8e);

		id[SYSCALL_ENTRY] = create_id(syscall as _, 0x8, 0xee);
	}

	let idt = InterruptDescriptorTable {
		size: (size_of::<InterruptDescriptor>() * ENTRIES_COUNT - 1) as u16,
		offset: unsafe { ID.assume_init_ref().as_ptr() as u32 },
	};
	unsafe {
		idt_load(&idt as *const _ as *const _);
	}
}

/// Tells whether interruptions are enabled.
pub fn is_interrupt_enabled() -> bool {
	unsafe { interrupt_is_enabled() != 0 }
}

/// Executes the given function `f` with maskable interruptions disabled.
///
/// This function saves the state of the interrupt flag and restores it before
/// returning.
pub fn wrap_disable_interrupts<T, F: FnOnce() -> T>(f: F) -> T {
	let int = is_interrupt_enabled();

	// Here is assumed that no interruption will change eflags. Which could cause a
	// race condition

	crate::cli!();

	let result = f();

	if int {
		crate::sti!();
	} else {
		crate::cli!();
	}

	result
}
