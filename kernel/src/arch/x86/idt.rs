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

use crate::{
	arch::{
		x86,
		x86::{cli, gdt, gdt::USER_CS64, pic, sti, DEFAULT_FLAGS},
	},
	syscall::syscall,
};
use core::{
	arch::{asm, global_asm},
	ffi::c_void,
	mem::size_of,
	ptr::addr_of,
};
use utils::errno::EResult;

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

/// Interruption stack frame, with saved registers state.
#[cfg(target_arch = "x86")]
#[repr(C)]
#[allow(missing_docs)]
#[derive(Default)]
pub struct IntFrame {
	// Using the prefix `r` to avoid duplicate code
	pub rax: u32,
	pub rbx: u32,
	pub rcx: u32,
	pub rdx: u32,
	pub rsi: u32,
	pub rdi: u32,
	pub rbp: u32,

	pub gs: u16,
	pub fs: u16,

	/// Interruption number.
	pub int: u32,
	/// Error code, if any.
	pub code: u32,

	pub rip: u32,
	pub cs: u32,
	pub rflags: u32,
	pub rsp: u32,
	pub ss: u32,
}

/// Interruption stack frame, with saved registers state.
#[cfg(target_arch = "x86_64")]
#[allow(missing_docs)]
#[repr(C)]
#[derive(Default)]
pub struct IntFrame {
	pub rax: u64,
	pub rbx: u64,
	pub rcx: u64,
	pub rdx: u64,
	pub rsi: u64,
	pub rdi: u64,
	pub rbp: u64,
	// Added by long mode
	pub r8: u64,
	pub r9: u64,
	pub r10: u64,
	pub r11: u64,
	pub r12: u64,
	pub r13: u64,
	pub r14: u64,
	pub r15: u64,

	// manual padding due to `gs` and `fs`
	_padding: u32,

	pub gs: u16,
	pub fs: u16,

	/// Interruption number.
	pub int: u64,
	/// Error code, if any.
	pub code: u64,

	pub rip: u64,
	pub cs: u64,
	pub rflags: u64,
	pub rsp: u64,
	pub ss: u64,
}

impl IntFrame {
	/// Tells whether the interrupted context is 32 bit.
	pub const fn is_32bit(&self) -> bool {
		self.cs as usize == gdt::USER_CS | 3
	}

	/// Returns the ID of the system call being executed.
	#[inline]
	pub const fn get_syscall_id(&self) -> usize {
		self.rax as usize
	}

	/// Returns the value of the `n`th argument of the syscall being executed.
	///
	/// If `n` exceeds the number of arguments for the current architecture, the function returns
	/// `0`.
	#[inline]
	pub const fn get_syscall_arg(&self, n: u8) -> usize {
		let val = if self.cs as usize & !0b11 == USER_CS64 {
			match n {
				0 => self.rdi,
				1 => self.rsi,
				2 => self.rdx,
				3 => self.r10,
				4 => self.r8,
				5 => self.r9,
				_ => 0,
			}
		} else {
			match n {
				0 => self.rbx,
				1 => self.rcx,
				2 => self.rdx,
				3 => self.rsi,
				4 => self.rdi,
				5 => self.rbp,
				_ => 0,
			}
		};
		val as _
	}

	/// Sets the return value of a system call.
	pub fn set_syscall_return(&mut self, value: EResult<usize>) {
		self.rax = value.map(|v| v as _).unwrap_or_else(|e| (-e.as_int()) as _);
	}

	/// Returns the stack address.
	pub fn get_stack_address(&self) -> usize {
		self.rsp as usize
	}

	/// Returns the address of the instruction to be executed when the interrupt handler returns.
	pub fn get_program_counter(&self) -> usize {
		self.rip as usize
	}

	/// Sets the address of the instruction to be executed when the interrupt handler returns.
	pub fn set_program_counter(&mut self, val: usize) {
		self.rip = val as _;
	}

	/// Sets the values of `frame` so that it can be used to begin the execution of a program.
	///
	/// Arguments:
	/// - `pc` is the program counter
	/// - `sp` is the stack pointer
	/// - `bit32` tells whether the program is 32 bits. If the kernel is compiled for 32 bit, this
	///   value is ignored.
	pub fn exec(frame: &mut Self, pc: usize, sp: usize, bit32: bool) {
		let cs_segment = if bit32 { gdt::USER_CS } else { gdt::USER_CS64 };
		*frame = IntFrame {
			rip: pc as _,
			cs: (cs_segment | 3) as _,
			rflags: DEFAULT_FLAGS as _,
			rsp: sp as _,
			ss: (gdt::USER_DS | 3) as _,
			..Default::default()
		};
	}
}

// include registers save/restore macros
#[cfg(target_arch = "x86")]
global_asm!(r#".include "arch/x86/src/regs.s""#);
#[cfg(target_arch = "x86_64")]
global_asm!(r#".include "arch/x86_64/src/regs.s""#);

/// An IDT header.
#[repr(C, packed)]
struct InterruptDescriptorTable {
	/// The size of the IDT in bytes, minus 1.
	size: u16,
	/// The address to the beginning of the IDT.
	#[cfg(target_arch = "x86")]
	offset: u32,
	/// The address to the beginning of the IDT.
	#[cfg(target_arch = "x86_64")]
	offset: u64,
}

/// An IDT entry.
#[repr(C)]
#[derive(Clone, Copy)]
struct InterruptDescriptor {
	/// Bits 0..16 of the address to the handler for the interrupt.
	offset0: u16,
	/// The code segment selector to execute the interrupt.
	selector: u16,
	/// Must be set to zero.
	zero0: u8,
	/// Interrupt handler flags.
	flags: u8,
	/// Bits 16..32 of the address to the handler for the interrupt.
	offset1: u16,
	/// Bits 32..64 of the address to the handler for the interrupt.
	#[cfg(target_arch = "x86_64")]
	offset2: u32,
	/// Must be set to zero.
	#[cfg(target_arch = "x86_64")]
	zero1: u32,
}

impl InterruptDescriptor {
	/// Returns a placeholder entry.
	///
	/// This function is necessary because the `const_trait_impl` feature is currently unstable,
	/// preventing to use `Default`.
	const fn placeholder() -> Self {
		Self {
			offset0: 0,
			selector: 0,
			zero0: 0,
			flags: 0,
			offset1: 0,
			#[cfg(target_arch = "x86_64")]
			offset2: 0,
			#[cfg(target_arch = "x86_64")]
			zero1: 0,
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
			offset0: (address as usize & 0xffff) as u16,
			selector,
			zero0: 0,
			flags,
			offset1: ((address as usize >> 16) & 0xffff) as u16,
			#[cfg(target_arch = "x86_64")]
			offset2: ((address as usize >> 32) & 0xffffffff) as u32,
			#[cfg(target_arch = "x86_64")]
			zero1: 0,
		}
	}
}

/// Declare an error handler.
///
/// An error can be accompanied by a code, in which case the handler must be declared with the
/// `code` keyword.
macro_rules! error {
	($name:ident, $id:expr) => {
		extern "C" {
			fn $name();
		}

		#[cfg(target_arch = "x86")]
		global_asm!(
			r"
.global {name}
.type {name}, @function

{name}:
	cld
	push 0 # code (absent)
	push {id}

STORE_REGS

	xor ebp, ebp
	push esp
	call interrupt_handler
	add esp, 4

LOAD_REGS
	add esp, 8
	iretd",
			name = sym $name,
			id = const($id)
		);

		#[cfg(target_arch = "x86_64")]
		global_asm!(
			r"
.global {name}
.type {name}, @function

{name}:
	cld
	push 0 # code (absent)
	push {id}
STORE_REGS

	xor rbp, rbp
	mov rdi, rsp
	call interrupt_handler

LOAD_REGS
	add rsp, 16
	iretq",
			name = sym $name,
			id = const($id)
		);
	};
	($name:ident, $id:expr, code) => {
		extern "C" {
			fn $name();
		}

		#[cfg(target_arch = "x86")]
		global_asm!(
			r#"
.global {name}
.type {name}, @function

{name}:
	cld
	push {id}
STORE_REGS

	xor ebp, ebp
	push esp
	call interrupt_handler
	add esp, 4

LOAD_REGS
	add esp, 8
	iretd"#,
			name = sym $name,
			id = const($id)
		);

		#[cfg(target_arch = "x86_64")]
		global_asm!(
			r#"
.global {name}
.type {name}, @function

{name}:
	cld
	push {id}
STORE_REGS

	xor rbp, rbp
	mov rdi, rsp
	call interrupt_handler

LOAD_REGS
	add rsp, 16
	iretq"#,
			name = sym $name,
			id = const($id)
		);
	};
}

macro_rules! irq {
	($name:ident, $id:expr) => {
		extern "C" {
			fn $name();
		}

		#[cfg(target_arch = "x86")]
		global_asm!(
			r#"
.global {name}

{name}:
	cld
	push 0 # code (absent)
	push {id}
STORE_REGS

	xor ebp, ebp
	push esp
	call interrupt_handler
	add esp, 4

LOAD_REGS
	add esp, 8
	iretd"#,
			name = sym $name,
			id = const($id)
		);

		#[cfg(target_arch = "x86_64")]
		global_asm!(
			r#"
.global {name}

{name}:
	cld
	push 0 # code (absent)
	push {id}
STORE_REGS

	xor rbp, rbp
	mov rdi, rsp
	call interrupt_handler

LOAD_REGS
	add rsp, 16
	iretq"#,
			name = sym $name,
			id = const($id)
		);
	};
}

error!(error0, 0x0);
error!(error1, 0x1);
error!(error2, 0x2);
error!(error3, 0x3);
error!(error4, 0x4);
error!(error5, 0x5);
error!(error6, 0x6);
error!(error7, 0x7);
error!(error8, 0x8, code);
error!(error9, 0x9);
error!(error10, 0xa, code);
error!(error11, 0xb, code);
error!(error12, 0xc, code);
error!(error13, 0xd, code);
error!(error14, 0xe, code);
error!(error15, 0xf);
error!(error16, 0x10);
error!(error17, 0x11, code);
error!(error18, 0x12);
error!(error19, 0x13);
error!(error20, 0x14);
error!(error21, 0x15);
error!(error22, 0x16);
error!(error23, 0x17);
error!(error24, 0x18);
error!(error25, 0x19);
error!(error26, 0x1a);
error!(error27, 0x1b);
error!(error28, 0x1c);
error!(error29, 0x1d);
error!(error30, 0x1e, code);
error!(error31, 0x1f);

irq!(irq0, 0x20);
irq!(irq1, 0x21);
irq!(irq2, 0x22);
irq!(irq3, 0x23);
irq!(irq4, 0x24);
irq!(irq5, 0x25);
irq!(irq6, 0x26);
irq!(irq7, 0x27);
irq!(irq8, 0x28);
irq!(irq9, 0x29);
irq!(irq10, 0x2a);
irq!(irq11, 0x2b);
irq!(irq12, 0x2c);
irq!(irq13, 0x2d);
irq!(irq14, 0x2e);
irq!(irq15, 0x2f);

/// The list of IDT entries.
static mut IDT_ENTRIES: [InterruptDescriptor; ENTRIES_COUNT] =
	[InterruptDescriptor::placeholder(); ENTRIES_COUNT];

/// Executes the given function `f` with maskable interruptions disabled.
///
/// This function saves the state of the interrupt flag and restores it before
/// returning.
pub fn wrap_disable_interrupts<T, F: FnOnce() -> T>(f: F) -> T {
	let int = x86::is_interrupt_enabled();
	// Here is assumed that no interruption will change flags register. Which could cause a
	// race condition
	cli();
	let res = f();
	if int {
		sti();
	} else {
		cli();
	}
	res
}

/// Initializes the IDT.
///
/// This function must be called only once at kernel initialization.
///
/// When returning, maskable interrupts are disabled by default.
pub fn init() {
	cli();
	pic::init(0x20, 0x28);
	// Safe because the current function is called only once at boot
	unsafe {
		// Errors
		IDT_ENTRIES[0x00] = InterruptDescriptor::new(error0 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x01] = InterruptDescriptor::new(error1 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x02] = InterruptDescriptor::new(error2 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x03] = InterruptDescriptor::new(error3 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x04] = InterruptDescriptor::new(error4 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x05] = InterruptDescriptor::new(error5 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x06] = InterruptDescriptor::new(error6 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x07] = InterruptDescriptor::new(error7 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x08] = InterruptDescriptor::new(error8 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x09] = InterruptDescriptor::new(error9 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x0a] = InterruptDescriptor::new(error10 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x0b] = InterruptDescriptor::new(error11 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x0c] = InterruptDescriptor::new(error12 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x0d] = InterruptDescriptor::new(error13 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x0e] = InterruptDescriptor::new(error14 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x0f] = InterruptDescriptor::new(error15 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x10] = InterruptDescriptor::new(error16 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x11] = InterruptDescriptor::new(error17 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x12] = InterruptDescriptor::new(error18 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x13] = InterruptDescriptor::new(error19 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x14] = InterruptDescriptor::new(error20 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x15] = InterruptDescriptor::new(error21 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x16] = InterruptDescriptor::new(error22 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x17] = InterruptDescriptor::new(error23 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x18] = InterruptDescriptor::new(error24 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x19] = InterruptDescriptor::new(error25 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x1a] = InterruptDescriptor::new(error26 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x1b] = InterruptDescriptor::new(error27 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x1c] = InterruptDescriptor::new(error28 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x1d] = InterruptDescriptor::new(error29 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x1e] = InterruptDescriptor::new(error30 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x1f] = InterruptDescriptor::new(error31 as _, 0x8, 0x8e);
		// IRQ
		IDT_ENTRIES[0x20] = InterruptDescriptor::new(irq0 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x21] = InterruptDescriptor::new(irq1 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x22] = InterruptDescriptor::new(irq2 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x23] = InterruptDescriptor::new(irq3 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x24] = InterruptDescriptor::new(irq4 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x25] = InterruptDescriptor::new(irq5 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x26] = InterruptDescriptor::new(irq6 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x27] = InterruptDescriptor::new(irq7 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x28] = InterruptDescriptor::new(irq8 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x29] = InterruptDescriptor::new(irq9 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x2a] = InterruptDescriptor::new(irq10 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x2b] = InterruptDescriptor::new(irq11 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x2c] = InterruptDescriptor::new(irq12 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x2d] = InterruptDescriptor::new(irq13 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x2e] = InterruptDescriptor::new(irq14 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x2f] = InterruptDescriptor::new(irq15 as _, 0x8, 0x8e);
		// System calls
		IDT_ENTRIES[SYSCALL_ENTRY] = InterruptDescriptor::new(syscall as _, 0x8, 0xee);
		// Load
		let idt = InterruptDescriptorTable {
			size: (size_of::<InterruptDescriptor>() * ENTRIES_COUNT - 1) as u16,
			offset: addr_of!(IDT_ENTRIES) as _,
		};
		asm!("lidt [{}]", in(reg) &idt);
	}
}
