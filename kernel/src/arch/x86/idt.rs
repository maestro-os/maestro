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

use super::{DEFAULT_FLAGS, cli, gdt, is_interrupt_enabled, sti};
use crate::syscall::syscall_int;
use core::{arch::asm, ffi::c_void, fmt, fmt::Formatter, mem::size_of, ptr::addr_of};
use utils::errno::EResult;

/// The IDT vector index for system calls.
pub const SYSCALL_ENTRY: usize = 0x80;
/// The number of entries into the IDT.
pub const ENTRIES_COUNT: usize = 0x81;

/// Interruption stack frame, with saved registers state.
#[cfg(target_arch = "x86")]
#[repr(C)]
#[allow(missing_docs)]
#[derive(Clone, Default)]
pub struct IntFrame {
	// Using the prefix `r` to avoid duplicate code
	pub rax: u32,
	pub rbx: u32,
	pub rcx: u32,
	pub rdx: u32,
	pub rsi: u32,
	pub rdi: u32,
	pub rbp: u32,

	pub gs: u32,
	pub fs: u32,

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
#[derive(Clone, Default)]
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

	pub gs: u64,
	pub fs: u64,

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
	/// Tells whether the interrupted context is in compatibility mode.
	pub const fn is_compat(&self) -> bool {
		self.cs as usize & !0b11 == gdt::USER_CS
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
		#[cfg(target_arch = "x86")]
		let val = match n {
			0 => self.rbx,
			1 => self.rcx,
			2 => self.rdx,
			3 => self.rsi,
			4 => self.rdi,
			5 => self.rbp,
			_ => 0,
		};
		#[cfg(target_arch = "x86_64")]
		let val = if self.cs as usize & !0b11 == gdt::USER_CS64 {
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
	/// - `compat` tells whether the program runs in compatibility mode. If the kernel is compiled
	///   for 32 bit, this value is ignored.
	pub fn exec(frame: &mut Self, pc: usize, sp: usize, compat: bool) {
		let cs_segment = if compat { gdt::USER_CS } else { gdt::USER_CS64 };
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

impl fmt::Display for IntFrame {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		const LEN: usize = size_of::<usize>() * 2;
		#[cfg(target_arch = "x86")]
		{
			f.write_fmt(format_args!("EAX: {:0LEN$x}", self.rax))?;
			f.write_fmt(format_args!(" EBX: {:0LEN$x}", self.rbx))?;
			f.write_fmt(format_args!(" ECX: {:0LEN$x}", self.rcx))?;
			f.write_fmt(format_args!(" EDX: {:0LEN$x}", self.rdx))?;
			f.write_fmt(format_args!(" ESI: {:0LEN$x}\n", self.rsi))?;
			f.write_fmt(format_args!("EDI: {:0LEN$x}", self.rdi))?;
			f.write_fmt(format_args!(" EBP: {:0LEN$x}", self.rbp))?;
			f.write_fmt(format_args!(" GS:  {:0LEN$x}", self.gs))?;
			f.write_fmt(format_args!(" FS:  {:0LEN$x}", self.fs))?;
			f.write_fmt(format_args!(" INT: {:0LEN$x}\n", self.int))?;
			f.write_fmt(format_args!("CODE: {:0LEN$x}", self.code))?;
			f.write_fmt(format_args!(" EIP: {:0LEN$x}", self.rip))?;
			f.write_fmt(format_args!(" CS: {:0LEN$x}", self.cs))?;
			f.write_fmt(format_args!(" EFL: {:0LEN$x}", self.rflags))?;
			f.write_fmt(format_args!(" ESP: {:0LEN$x}\n", self.rsp))?;
			f.write_fmt(format_args!("SS: {:0LEN$x}", self.ss))?;
		}
		#[cfg(target_arch = "x86_64")]
		{
			f.write_fmt(format_args!("RAX: {:0LEN$x}", self.rax))?;
			f.write_fmt(format_args!(" RBX: {:0LEN$x}", self.rbx))?;
			f.write_fmt(format_args!(" RCX: {:0LEN$x}\n", self.rcx))?;
			f.write_fmt(format_args!("RDX: {:0LEN$x}", self.rdx))?;
			f.write_fmt(format_args!(" RSI: {:0LEN$x}", self.rsi))?;
			f.write_fmt(format_args!(" RDI: {:0LEN$x}\n", self.rdi))?;
			f.write_fmt(format_args!("RBP: {:0LEN$x}", self.rbp))?;
			f.write_fmt(format_args!(" R8:  {:0LEN$x}", self.r8))?;
			f.write_fmt(format_args!(" R9:  {:0LEN$x}\n", self.r9))?;
			f.write_fmt(format_args!("R10: {:0LEN$x}", self.r10))?;
			f.write_fmt(format_args!(" R11: {:0LEN$x}", self.r11))?;
			f.write_fmt(format_args!(" R12: {:0LEN$x}\n", self.r12))?;
			f.write_fmt(format_args!("R13: {:0LEN$x}", self.r13))?;
			f.write_fmt(format_args!(" R14: {:0LEN$x}", self.r12))?;
			f.write_fmt(format_args!(" R15: {:0LEN$x}\n", self.r15))?;
			f.write_fmt(format_args!("GS:  {:0LEN$x}", self.gs))?;
			f.write_fmt(format_args!(" FS:  {:0LEN$x}", self.fs))?;
			f.write_fmt(format_args!(" INT: {:0LEN$x}\n", self.int))?;
			f.write_fmt(format_args!("CODE:   {:0LEN$x}", self.code))?;
			f.write_fmt(format_args!(" RIP: {:0LEN$x}", self.rip))?;
			f.write_fmt(format_args!(" CS: {:0LEN$x}\n", self.cs))?;
			f.write_fmt(format_args!("RFL: {:0LEN$x}", self.rflags))?;
			f.write_fmt(format_args!(" RSP: {:0LEN$x}", self.rsp))?;
			f.write_fmt(format_args!(" SS: {:0LEN$x}", self.ss))?;
		}
		Ok(())
	}
}

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
	/// - `flags` is the set of flags for the entry (see Intel documentation).
	fn new(address: *const c_void, flags: u8) -> Self {
		Self {
			offset0: (address as usize & 0xffff) as u16,
			selector: 8, // kernel code segment
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

unsafe extern "C" {
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

	fn idt_ignore();
}

/// The list of IDT entries.
static mut IDT_ENTRIES: [InterruptDescriptor; ENTRIES_COUNT] =
	[InterruptDescriptor::placeholder(); ENTRIES_COUNT];

/// Executes the given function `f` with maskable interruptions disabled.
///
/// This function saves the state of the interrupt flag and restores it before
/// returning.
pub fn disable_int<T, F: FnOnce() -> T>(f: F) -> T {
	let int = is_interrupt_enabled();
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

/// Fills the IDT, which is common to all CPU cores.
///
/// This function must be called only once at kernel initialization.
pub(crate) fn init_table() {
	// Safe because the current function is called only once at boot
	unsafe {
		// Fill with default entries
		#[allow(static_mut_refs)] // No one else is accessing this
		IDT_ENTRIES.fill(InterruptDescriptor::new(idt_ignore as _, 0x8e));
		// Errors
		IDT_ENTRIES[0x00] = InterruptDescriptor::new(error0 as _, 0x8e);
		IDT_ENTRIES[0x01] = InterruptDescriptor::new(error1 as _, 0x8e);
		IDT_ENTRIES[0x02] = InterruptDescriptor::new(error2 as _, 0x8e);
		IDT_ENTRIES[0x03] = InterruptDescriptor::new(error3 as _, 0x8e);
		IDT_ENTRIES[0x04] = InterruptDescriptor::new(error4 as _, 0x8e);
		IDT_ENTRIES[0x05] = InterruptDescriptor::new(error5 as _, 0x8e);
		IDT_ENTRIES[0x06] = InterruptDescriptor::new(error6 as _, 0x8e);
		IDT_ENTRIES[0x07] = InterruptDescriptor::new(error7 as _, 0x8e);
		IDT_ENTRIES[0x08] = InterruptDescriptor::new(error8 as _, 0x8e);
		IDT_ENTRIES[0x09] = InterruptDescriptor::new(error9 as _, 0x8e);
		IDT_ENTRIES[0x0a] = InterruptDescriptor::new(error10 as _, 0x8e);
		IDT_ENTRIES[0x0b] = InterruptDescriptor::new(error11 as _, 0x8e);
		IDT_ENTRIES[0x0c] = InterruptDescriptor::new(error12 as _, 0x8e);
		IDT_ENTRIES[0x0d] = InterruptDescriptor::new(error13 as _, 0x8e);
		IDT_ENTRIES[0x0e] = InterruptDescriptor::new(error14 as _, 0x8e);
		IDT_ENTRIES[0x0f] = InterruptDescriptor::new(error15 as _, 0x8e);
		IDT_ENTRIES[0x10] = InterruptDescriptor::new(error16 as _, 0x8e);
		IDT_ENTRIES[0x11] = InterruptDescriptor::new(error17 as _, 0x8e);
		IDT_ENTRIES[0x12] = InterruptDescriptor::new(error18 as _, 0x8e);
		IDT_ENTRIES[0x13] = InterruptDescriptor::new(error19 as _, 0x8e);
		IDT_ENTRIES[0x14] = InterruptDescriptor::new(error20 as _, 0x8e);
		IDT_ENTRIES[0x15] = InterruptDescriptor::new(error21 as _, 0x8e);
		IDT_ENTRIES[0x16] = InterruptDescriptor::new(error22 as _, 0x8e);
		IDT_ENTRIES[0x17] = InterruptDescriptor::new(error23 as _, 0x8e);
		IDT_ENTRIES[0x18] = InterruptDescriptor::new(error24 as _, 0x8e);
		IDT_ENTRIES[0x19] = InterruptDescriptor::new(error25 as _, 0x8e);
		IDT_ENTRIES[0x1a] = InterruptDescriptor::new(error26 as _, 0x8e);
		IDT_ENTRIES[0x1b] = InterruptDescriptor::new(error27 as _, 0x8e);
		IDT_ENTRIES[0x1c] = InterruptDescriptor::new(error28 as _, 0x8e);
		IDT_ENTRIES[0x1d] = InterruptDescriptor::new(error29 as _, 0x8e);
		IDT_ENTRIES[0x1e] = InterruptDescriptor::new(error30 as _, 0x8e);
		IDT_ENTRIES[0x1f] = InterruptDescriptor::new(error31 as _, 0x8e);
		// IRQ
		IDT_ENTRIES[0x20] = InterruptDescriptor::new(irq0 as _, 0x8e);
		IDT_ENTRIES[0x21] = InterruptDescriptor::new(irq1 as _, 0x8e);
		IDT_ENTRIES[0x22] = InterruptDescriptor::new(irq2 as _, 0x8e);
		IDT_ENTRIES[0x23] = InterruptDescriptor::new(irq3 as _, 0x8e);
		IDT_ENTRIES[0x24] = InterruptDescriptor::new(irq4 as _, 0x8e);
		IDT_ENTRIES[0x25] = InterruptDescriptor::new(irq5 as _, 0x8e);
		IDT_ENTRIES[0x26] = InterruptDescriptor::new(irq6 as _, 0x8e);
		IDT_ENTRIES[0x27] = InterruptDescriptor::new(irq7 as _, 0x8e);
		IDT_ENTRIES[0x28] = InterruptDescriptor::new(irq8 as _, 0x8e);
		IDT_ENTRIES[0x29] = InterruptDescriptor::new(irq9 as _, 0x8e);
		IDT_ENTRIES[0x2a] = InterruptDescriptor::new(irq10 as _, 0x8e);
		IDT_ENTRIES[0x2b] = InterruptDescriptor::new(irq11 as _, 0x8e);
		IDT_ENTRIES[0x2c] = InterruptDescriptor::new(irq12 as _, 0x8e);
		IDT_ENTRIES[0x2d] = InterruptDescriptor::new(irq13 as _, 0x8e);
		IDT_ENTRIES[0x2e] = InterruptDescriptor::new(irq14 as _, 0x8e);
		IDT_ENTRIES[0x2f] = InterruptDescriptor::new(irq15 as _, 0x8e);
		// System calls
		IDT_ENTRIES[SYSCALL_ENTRY] = InterruptDescriptor::new(syscall_int as _, 0xee);
	}
}

/// Enables the syscall/sysret instruction pairs if available.
#[cfg(target_arch = "x86_64")]
fn enable_syscall_inst() {
	use super::cpuid::cpuid;

	let (_, _, _, mask) = cpuid(0x80000001, 0);
	let available = mask & (1 << 11) != 0;
	if !available {
		return;
	}
	// STAR
	super::wrmsr(
		0xc0000081,
		((gdt::KERNEL_CS as u64) << 32) | ((gdt::USER_CS as u64) << 48),
	);
	// LSTAR
	super::wrmsr(0xc0000082, crate::syscall::syscall as usize as u64);
	// SFMASK (clear direction and interrupt flag)
	super::wrmsr(0xc0000084, 0x600);
}

/// Binds the IDT to the current CPU core.
///
/// When returning, maskable interrupts are disabled by default.
pub(crate) fn bind() {
	unsafe {
		cli();
		let idt = InterruptDescriptorTable {
			size: (size_of::<InterruptDescriptor>() * ENTRIES_COUNT - 1) as u16,
			offset: addr_of!(IDT_ENTRIES) as _,
		};
		asm!("lidt [{}]", in(reg) &idt);
		#[cfg(target_arch = "x86_64")]
		enable_syscall_inst();
	}
}
