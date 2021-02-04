/// Under the x86 architecture, the TSS (Task State Segment) is a structure that is mostly
/// deprecated but that must still be used in order to perform software context switching because
/// it allows to store the pointers to the stacks to use whenever an interruption happens and
/// requires switching the protection ring, and thus the stack.
/// The structure has to be registered into the GDT into the TSS segment, and must be loaded using
/// instruction `ltr`.

use core::mem::size_of;
use crate::gdt;

/// The TSS structure.
#[repr(C, packed)]
struct TSSEntry {
	prev_tss: u32,
	esp0: u32,
	ss0: u32,
	esp1: u32,
	ss1: u32,
	esp2: u32,
	ss2: u32,
	cr3: u32,
	eip: u32,
	eflags: u32,
	eax: u32,
	ecx: u32,
	edx: u32,
	ebx: u32,
	esp: u32,
	ebp: u32,
	esi: u32,
	edi: u32,
	es: u32,
	cs: u32,
	ss: u32,
	ds: u32,
	fs: u32,
	gs: u32,
	ldt: u32,
	trap: u16,
	iomap_base: u16,
}

extern "C" {
	fn tss_get() -> *mut u64;
	fn tss_flush();
}

/// x86. Initializes the TSS.
pub fn init() {
	let tss_ptr = gdt::get_segment_ptr(gdt::TSS_OFFSET);

	let limit = size_of::<TSSEntry>() as u64;
	let base = unsafe { // Call to ASM function
		tss_get() as u64
	};
	let flags = 0b0100000010001001 as u64;
	let tss_value = (limit & 0xffff)
		| ((base & 0xffffff) << 16)
		| (flags << 40)
		| ((base & 0xff) << 56);

	unsafe { // Derference of raw pointer
		*tss_ptr = tss_value;
	}
}

/// x86. Updates the TSS into the GDT.
#[inline(always)]
pub fn flush() {
	unsafe { // Call to C function
		tss_flush();
	}
}
