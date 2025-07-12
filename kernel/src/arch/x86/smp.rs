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

//! Symmetric MultiProcessing management.

use super::apic::{
	REG_ERROR_STATUS, REG_ICR_HI, REG_ICR_LO, lapic_id, read_reg, wait_delivery, write_reg,
};
use crate::{
	arch::x86::apic,
	boot::BOOT_STACK_SIZE,
	memory::{PhysAddr, malloc::__alloc},
	process::scheduler::Cpu,
};
use core::{alloc::Layout, arch::global_asm, ffi::c_void, ptr::null_mut};
use utils::{errno::AllocResult, vec};

/// The SMP trampoline's physical address in memory.
const TRAMPOLINE_PHYS_ADDR: PhysAddr = PhysAddr(0x8000);

global_asm!(
	r"
.section .text
.code16

smp_trampoline:
	cli
	cld
	ljmp 0, 0x8040

	# GDT
.align 16
_gdt_table:
	.long 0, 0
	.long 0x0000ffff, 0x00cf9a00 # code
	.long 0x0000ffff, 0x008f9200 # data
	.long 0x00000068, 0x00cf8900 # tss
_gdt:
	.word _gdt - _gdt_table - 1
	.long 0x8010
	.long 0, 0
.align 64

	# Setup GDT
	xor ax, ax
	mov ds, ax
	lgdt [0x8030]
	mov eax, cr0
	or eax, 1
	mov cr0, eax
	ljmp 8, 0x8060

.align 32
.code32

	mov ax, 16
	mov ds, ax
	mov ss, ax

	# Get Local APIC ID
	mov eax, 1
	cpuid
	shr ebx, 24
	mov edi, ebx

	# Setup local stack
	shl ebx, 15
	mov esp, 0 # TODO {SMP_STACK_TOP}
	sub ebx, esp
	push edi

	# TODO if 64 bit, setup long mode
	# TODO relocate to higher memory
	# TODO switch to per-core GDT (because of thread locals)
	# TODO jump to Rust code
smp_trampoline_end:
",
	SMP_STACK_TOP = sym SMP_STACK_TOP
);

unsafe extern "C" {
	fn smp_trampoline();
	fn smp_trampoline_end();
}

/// An array of pointers to the top of stacks for each core to boot.
static mut SMP_STACK_TOP: *const *mut c_void = null_mut();

/// Initializes the SMP.
///
/// `cpu` is the list of CPU cores on the system.
pub fn init(cpu: &[Cpu]) -> AllocResult<()> {
	let lapic_id = lapic_id();
	let base_addr = PhysAddr(apic::get_base_addr())
		.kernel_to_virtual()
		.unwrap()
		.as_ptr();
	// Copy trampoline code
	unsafe {
		let trampoline_ptr: *mut u8 = TRAMPOLINE_PHYS_ADDR.kernel_to_virtual().unwrap().as_ptr();
		trampoline_ptr.copy_from(
			smp_trampoline as *const _,
			smp_trampoline_end as *const () as usize - smp_trampoline as *const () as usize,
		);
	}
	// Allocate stacks list
	let max_apic_id = cpu
		.iter()
		.map(|c| c.apic_id as usize + 1)
		.max()
		.unwrap_or(0);
	let mut stacks = vec![null_mut(); max_apic_id]?;
	unsafe {
		SMP_STACK_TOP = stacks.as_ptr();
	}
	let stack_layout = Layout::array::<u8>(BOOT_STACK_SIZE).unwrap();
	// Boot cores
	for cpu in cpu {
		// Do no attempt to boot the current core
		if cpu.apic_id == lapic_id {
			continue;
		}
		// Allocate stack
		unsafe {
			let stack = __alloc(stack_layout)?.cast();
			stacks[cpu.apic_id as usize] = stack.add(BOOT_STACK_SIZE).as_ptr();
		}
		// Send INIT IPI
		unsafe {
			// Clear APIC error
			write_reg(base_addr, REG_ERROR_STATUS, 0);
			// Select AP
			write_reg(
				base_addr,
				REG_ICR_HI,
				(read_reg(base_addr, REG_ICR_HI) & 0x00ffffff) | ((cpu.apic_id as u32) << 24),
			);
			// Trigger INIT IPI
			write_reg(
				base_addr,
				REG_ICR_LO,
				(read_reg(base_addr, REG_ICR_LO) & 0xfff00000) | 0xc500,
			);
			wait_delivery(base_addr);
			// Select AP
			write_reg(
				base_addr,
				REG_ICR_HI,
				(read_reg(base_addr, REG_ICR_HI) & 0x00ffffff) | ((cpu.apic_id as u32) << 24),
			);
			// INIT de-assert
			write_reg(
				base_addr,
				REG_ICR_LO,
				(read_reg(base_addr, REG_ICR_LO) & 0xfff00000) | 0x8500,
			);
			wait_delivery(base_addr);
		}
		// TODO 10 msec delay
		// Send startup IPI twice
		for _ in 0..2 {
			unsafe {
				// Clear APIC error
				write_reg(base_addr, REG_ERROR_STATUS, 0);
				// Select AP
				write_reg(
					base_addr,
					REG_ICR_HI,
					(read_reg(base_addr, REG_ICR_HI) & 0x00ffffff) | ((cpu.apic_id as u32) << 24),
				);
				// Trigger STARTUP IPI
				write_reg(
					base_addr,
					REG_ICR_LO,
					(read_reg(base_addr, REG_ICR_LO) & 0xfff0f800) | 0x608,
				);
				// TODO wait for 200 usec
				wait_delivery(base_addr);
			}
		}
	}
	Ok(())
}
