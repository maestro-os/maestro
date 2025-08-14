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

//! Context switching utilities.

use crate::{
	arch::x86::{fxrstor, fxsave, gdt, idt::IntFrame},
	memory::vmem::KERNEL_VMEM,
	process::{Process, mem_space::MemSpace, scheduler::core_local},
};
use core::{arch::global_asm, mem::offset_of, ptr::NonNull};

/// Saves the current FS and GS values to `proc`.
pub fn save_segments(proc: &Process) {
	#[cfg(not(target_arch = "x86_64"))]
	let _ = proc;
	#[cfg(target_arch = "x86_64")]
	{
		use crate::arch::x86;
		use core::{arch::asm, sync::atomic::Ordering::Relaxed};

		proc.fs_base.store(x86::rdmsr(x86::IA32_FS_BASE), Relaxed);
		proc.gs_base
			.store(x86::rdmsr(x86::IA32_KERNEL_GS_BASE), Relaxed);
		let mut fs: u16;
		let mut gs: u16;
		unsafe {
			asm!(
				"mov {fs:x}, fs",
				"mov {gs:x}, gs",
				fs = out(reg) fs,
				gs = out(reg) gs
			);
		}
		proc.fs_selector.store(fs, Relaxed);
		proc.gs_selector.store(gs, Relaxed);
	}
}

/// Restores the FS and GS values from `proc` to the current context.
pub fn restore_segments(proc: &Process) {
	#[cfg(not(target_arch = "x86_64"))]
	let _ = proc;
	#[cfg(target_arch = "x86_64")]
	{
		use crate::arch::x86;
		use core::{arch::asm, sync::atomic::Ordering::Relaxed};

		// Stash to prevent zeroing when settings `gs`
		let gs_base = x86::rdmsr(x86::IA32_GS_BASE);
		// Restore segment selectors
		let fs = proc.fs_selector.load(Relaxed);
		let gs = proc.gs_selector.load(Relaxed);
		unsafe {
			asm!(
				"mov fs, {fs:x}",
				"mov gs, {gs:x}",
				fs = in(reg) fs,
				gs = in(reg) gs
			);
		}
		x86::wrmsr(x86::IA32_GS_BASE, gs_base);
		// Restore bases
		let fs_base = proc.fs_base.load(Relaxed);
		let gs_base = proc.gs_base.load(Relaxed);
		x86::wrmsr(x86::IA32_FS_BASE, fs_base);
		x86::wrmsr(x86::IA32_KERNEL_GS_BASE, gs_base);
	}
}

// Note: the functions below are saving only the registers that are not clobbered by the call to
// them

unsafe extern "C" {
	/// Jumps to a new context with the given `frame`.
	///
	/// # Safety
	///
	/// The context described by `frame` must be valid.
	pub fn init_ctx(frame: &IntFrame) -> !;
	/// The idle task code.
	pub fn idle_task() -> !;

	/// Switches context from `prev` to `next`.
	///
	/// # Safety
	///
	/// The pointers must point to valid processes.
	#[allow(improper_ctypes)]
	pub fn switch(prev: *const Process, next: *const Process);

	/// Trampoline for launching a new process after a fork.
	fn fork_trampoline();
	/// Trampoline prepare for launching a new kernel thread.
	fn kthread_trampoline();
}

#[cfg(target_arch = "x86")]
global_asm!(r#"
.section .text

.global switch
.global fork_trampoline
.global kthread_trampoline
.type switch, @function
.type fork_trampoline, @function
.type kthread_trampoline, @function

switch:
	# Preserve arguments across stack switch
	mov eax, [esp + 4]
	mov edx, [esp + 8]
	
	push ebp
	push ebx
	push esi
	push edi

    # Swap contexts
    mov eax, [esp + 20]
    mov [eax + {off}], esp
    mov eax, [esp + 24]
    mov esp, [eax + {off}]

	pop edi
	pop esi
	pop ebx
	pop ebp

	mov [esp + 4], eax
	mov [esp + 8], edx
	jmp switch_finish

fork_trampoline:
	# Remove unused arguments to `switch`
	add esp, 8
	push esp
	call init_ctx
	
kthread_trampoline:
	# Remove arguments to switch
	add esp, 8
	
	# Clear segment selectors
	xor bx, bx
	mov fs, bx
	mov gs, bx

	# Jump to entry point
	ret
"#, off = const offset_of!(Process, kernel_sp));

#[cfg(target_arch = "x86_64")]
global_asm!(r#"
.section .text

.global switch
.global fork_trampoline
.global kthread_trampoline
.type switch, @function
.type fork_trampoline, @function
.type kthread_trampoline, @function

switch:
	push rbp
	push rbx
	push r12
	push r13
	push r14
	push r15

    # Swap contexts
    mov [rdi + {off}], rsp
    mov rsp, [rsi + {off}]

	pop r15
	pop r14
	pop r13
	pop r12
	pop rbx
	pop rbp

	jmp switch_finish

fork_trampoline:
	mov rdi, rsp
	jmp init_ctx

kthread_trampoline:
	# Save GS base
	mov ecx, 0xc0000101
	rdmsr

	# Clear segment selectors
	xor bx, bx
	mov fs, bx
	mov gs, bx

	# Restore GS base
	mov ecx, 0xc0000101
	wrmsr

	# Zero FS base and Kernel GS base
	xor eax, eax
	xor edx, edx
	mov ecx, 0xc0000100
	wrmsr
	mov ecx, 0xc0000102
	wrmsr

	# Jump to entry point
	ret
"#, off = const offset_of!(Process, kernel_sp));

/// Finishes switching context from `prev` to `next`, that is restore everything else than
/// general-purpose registers.
///
/// This function is jumped to from [`switch`].
#[unsafe(export_name = "switch_finish")]
pub extern "C" fn finish(prev: &Process, next: &Process) {
	// TODO save and restore only if necessary (enable the FPU when the first interruption occurs)
	// Switch FPU state
	fxsave(&mut prev.fpu.lock());
	fxrstor(&next.fpu.lock());
	// Save segments
	save_segments(prev);
	// Restore TLS entries from `next`
	next.tls
		.lock()
		.iter()
		.enumerate()
		.for_each(|(i, ent)| unsafe {
			ent.update_gdt(gdt::TLS_OFFSET + i * size_of::<gdt::Entry>());
		});
	// Bind memory space
	match next.mem_space.as_ref() {
		Some(mem_space) => MemSpace::bind(mem_space),
		// No associated memory context: bind the kernel's
		None => KERNEL_VMEM.lock().bind(),
	}
	// Restore segments
	restore_segments(next);
	// Update the TSS for the process
	unsafe {
		core_local()
			.tss()
			.set_kernel_stack(next.kernel_stack.top().as_ptr());
	}
}

#[cfg(target_arch = "x86")]
#[repr(C, packed)]
struct ForkFrame {
	/// Padding for unused registers pop
	pad: [u8; 16],
	/// `fork_trampoline` address
	trampoline: u32,
	/// Space for the arguments to [`switch`]
	args: [u32; 2],
	/// Initial frame
	frame: IntFrame,
}

#[cfg(target_arch = "x86_64")]
#[repr(C, packed)]
struct ForkFrame {
	/// Padding for unused registers pop
	pad: [u8; 48],
	/// `fork_trampoline` address
	trampoline: u64,
	/// Initial frame
	frame: IntFrame,
}

/// Writes a forking frame on `stack`.
///
/// `frame` is the register state the process begins with.
///
/// The function returns the new stack pointer with the frame on top.
///
/// # Safety
///
/// `stack` must be the top of a valid stack.
pub unsafe fn init_fork(stack: NonNull<u8>, frame: IntFrame) -> *mut u8 {
	#[cfg(target_arch = "x86")]
	let frame = ForkFrame {
		pad: [0; 16],
		trampoline: fork_trampoline as *const () as _,
		args: [0; 2],
		frame,
	};
	#[cfg(target_arch = "x86_64")]
	let frame = ForkFrame {
		pad: [0; 48],
		trampoline: fork_trampoline as *const () as _,
		frame,
	};
	let stack = stack.cast().sub(1);
	stack.write(frame);
	stack.cast().as_ptr()
}

/// The entry point of a kernel thread.
pub type KThreadEntry = fn() -> !;

#[cfg(target_arch = "x86")]
#[repr(C, packed)]
struct KThreadInit {
	/// Padding for unused registers pop
	pad: [u8; 16],
	/// `kthread_trampoline` address
	trampoline: u32,
	/// Space for the arguments to [`switch`]
	args: [u32; 2],
	/// The thread's entry point
	entry: u32,
}

#[cfg(target_arch = "x86_64")]
#[repr(C, packed)]
struct KThreadInit {
	/// Padding for unused registers pop
	pad: [u8; 48],
	/// `kthread_trampoline` address
	trampoline: u64,
	/// The thread's entry point
	entry: u64,
}

/// Writes an initialization frame for a kernel thread on `stack`.
///
/// The function returns the new stack pointer with the frame on top.
///
/// # Safety
///
/// `stack` must be the top of a valid stack.
pub unsafe fn init_kthread(stack: NonNull<u8>, entry: KThreadEntry) -> *mut u8 {
	#[cfg(target_arch = "x86")]
	let frame = KThreadInit {
		pad: [0; 16],
		trampoline: kthread_trampoline as *const () as _,
		args: [0; 2],
		entry: entry as *const () as _,
	};
	#[cfg(target_arch = "x86_64")]
	let frame = KThreadInit {
		pad: [0; 48],
		trampoline: kthread_trampoline as *const () as _,
		entry: entry as *const () as _,
	};
	let stack = stack.cast().sub(1);
	stack.write(frame);
	stack.cast().as_ptr()
}
