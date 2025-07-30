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
	arch,
	arch::x86::{apic, gdt, tss},
	boot::BOOT_STACK_SIZE,
	memory::{
		PhysAddr, VirtAddr, buddy,
		vmem::{KERNEL_VMEM, write_ro},
	},
	println,
	process::scheduler::{Cpu, init_core_local, switch::idle_task},
};
use core::{
	arch::global_asm,
	hint, ptr,
	ptr::null_mut,
	sync::atomic::{AtomicUsize, Ordering::Acquire},
};
use utils::{collections::vec::Vec, errno::AllocResult, limits::PAGE_SIZE, vec};

/// The SMP trampoline's physical address in memory.
pub const TRAMPOLINE_PHYS_ADDR: PhysAddr = PhysAddr(0x8000);

#[cfg(target_arch = "x86")]
global_asm!(
	r"
.section .text
.code16

.global smp_trampoline
.global smp_trampoline_end

.set SMP_VAR_ADDR, 0x8000 + (smp_trampoline_end - smp_trampoline)

smp_trampoline:
	cli
	cld
	ljmp 0, 0x8040

.align 16
_gdt_table:
	.long 0, 0
	.long 0x0000ffff, 0x00cf9a00 # code
	.long 0x0000ffff, 0x008f9200 # data
_gdt:
	.word _gdt - _gdt_table - 1
	.long 0x8010
	.long 0, 0
.align 64

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

    # Set page directory
    mov eax, [SMP_VAR_ADDR]
	mov cr3, eax

    # Enable PSE
	mov eax, cr4
	or eax, 0x10
	mov cr4, eax

	# Enable paging and write protect
	or eax, 0x80010000
	mov cr0, eax

	# Setup local stack
	mov esp, [SMP_VAR_ADDR + 8]
	shl ebx, 3
	add esp, ebx
	mov esp, [esp]

	push 0
	popfd

	jmp smp_main

.align 8
smp_trampoline_end:
"
);

#[cfg(target_arch = "x86_64")]
global_asm!(
	r"
.section .text
.code16

.global smp_trampoline
.global smp_trampoline_end

.set SMP_VAR_ADDR, 0x8000 + (smp_trampoline_end - smp_trampoline)

smp_trampoline:
	cli
	cld
	ljmp 0, 0x8040

.align 16
_gdt_table:
	.long 0, 0
	.long 0x0000ffff, 0x00af9a00 # code 64
	.long 0x0000ffff, 0x008f9200 # data
	.long 0x0000ffff, 0x00cf9a00 # code 32
_gdt:
	.word _gdt - _gdt_table - 1
	.long 0x8010
	.long 0, 0
.align 64

	xor ax, ax
	mov ds, ax
	lgdt [0x8030]
	mov eax, cr0
	or eax, 1
	mov cr0, eax
	ljmp 24, 0x8060

.align 32
.code32
	mov ax, 16
	mov ds, ax
	mov ss, ax

	# Get Local APIC ID
	mov eax, 1
	cpuid
	shr ebx, 24

    # Set PDPT
    mov eax, [SMP_VAR_ADDR]
	mov cr3, eax

	# Enable PSE and PAE
	mov eax, cr4
	or eax, 0x30
	mov cr4, eax

	# Enable LME
	mov ecx, 0xc0000080 # EFER
	rdmsr
	or eax, 0x901
	wrmsr

	# Enable paging and write protect
	or eax, 0x80010000
	mov cr0, eax

	ljmp 8, 0x80a2

.code64
	# Setup local stack
	mov rsp, [SMP_VAR_ADDR + 8]
	shl rbx, 3
	add rsp, rbx
	mov rsp, [rsp]

	push 0
	popfq

	movabs rax, offset smp_main
	jmp rax

.align 8
smp_trampoline_end:
"
);

unsafe extern "C" {
	fn smp_trampoline();
	fn smp_trampoline_end();
}

/// The number of running CPU cores.
static BOOTED_CORES: AtomicUsize = AtomicUsize::new(1);

/// Initializes the SMP.
///
/// `cpu` is the list of CPU cores on the system.
pub fn init(cpu: &[Cpu]) -> AllocResult<()> {
	let lapic_id = lapic_id();
	let base_addr = PhysAddr(apic::get_base_addr())
		.kernel_to_virtual()
		.unwrap()
		.as_ptr();
	// Allocate stacks list
	let max_apic_id = cpu
		.iter()
		.map(|c| c.apic_id as usize + 1)
		.max()
		.unwrap_or(0);
	let mut stacks: Vec<*mut u8> = vec![null_mut(); max_apic_id]?;
	// Copy trampoline code
	let trampoline_ptr: *mut u8 = TRAMPOLINE_PHYS_ADDR.kernel_to_virtual().unwrap().as_ptr();
	let trampoline_len =
		smp_trampoline_end as *const () as usize - smp_trampoline as *const () as usize;
	unsafe {
		write_ro(|| {
			trampoline_ptr.copy_from(smp_trampoline as *const _, trampoline_len);
			// Pass pointers to the trampoline
			let ptrs = trampoline_ptr.add(trampoline_len).cast();
			let vmem_phys = VirtAddr::from(KERNEL_VMEM.lock().inner().as_ptr())
				.kernel_to_physical()
				.unwrap();
			ptr::write_volatile(ptrs, vmem_phys.0 as u64);
			ptr::write_volatile(ptrs.add(1), stacks.as_ptr() as u64);
		});
	}
	// Boot cores
	for cpu in cpu {
		// Do no attempt to boot the current core
		if cpu.apic_id == lapic_id {
			continue;
		}
		// Allocate stack
		unsafe {
			let order = buddy::get_order(BOOT_STACK_SIZE / PAGE_SIZE);
			let stack = buddy::alloc_kernel(order, 0)?.cast();
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
	// Wait for all cores to be up before returning
	while BOOTED_CORES.load(Acquire) < cpu.len() {
		hint::spin_loop();
	}
	Ok(())
}

/// First function called after the SMP trampoline
#[unsafe(no_mangle)]
unsafe extern "C" fn smp_main() -> ! {
	arch::init1(false);
	// TODO call init2? need to setup APIC? need to calibrate the APIC timer
	init_core_local();
	gdt::flush();
	tss::init();
	println!("started core {}!", lapic_id());
	BOOTED_CORES.fetch_add(1, Acquire);
	// Wait for work
	unsafe {
		idle_task();
	}
}
