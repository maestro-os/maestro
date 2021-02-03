/// TODO doc

use core::mem::size_of;
use crate::gdt;

/// TODO doc
#[repr(C, packed)]
struct TSSEntry {
	prev_tss: i32,
	esp0: i32,
	ss0: i32,
	esp1: i32,
	ss1: i32,
	esp2: i32,
	ss2: i32,
	cr3: i32,
	eip: i32,
	eflags: i32,
	eax: i32,
	ecx: i32,
	edx: i32,
	ebx: i32,
	esp: i32,
	ebp: i32,
	esi: i32,
	edi: i32,
	es: i32,
	cs: i32,
	ss: i32,
	ds: i32,
	fs: i32,
	gs: i32,
	ldt: i32,
	trap: i16,
	iomap_base: i16,
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
