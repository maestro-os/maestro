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
pub struct TSSEntry {
	pub prev_tss: u32,
	pub esp0: u32,
	pub ss0: u32,
	pub esp1: u32,
	pub ss1: u32,
	pub esp2: u32,
	pub ss2: u32,
	pub cr3: u32,
	pub eip: u32,
	pub eflags: u32,
	pub eax: u32,
	pub ecx: u32,
	pub edx: u32,
	pub ebx: u32,
	pub esp: u32,
	pub ebp: u32,
	pub esi: u32,
	pub edi: u32,
	pub es: u32,
	pub cs: u32,
	pub ss: u32,
	pub ds: u32,
	pub fs: u32,
	pub gs: u32,
	pub ldt: u32,
	pub trap: u16,
	pub iomap_base: u16,
}

extern "C" {
	fn tss_get() -> *mut TSSEntry;
	fn tss_flush();
}

/// x86. Initializes the TSS.
pub fn init() {
	let tss_ptr = gdt::get_segment_ptr(gdt::TSS_OFFSET);

	let limit = size_of::<TSSEntry>() as u64;
	let base = unsafe {
		tss_get() as u64
	};
	let flags = 0b0100000010001001 as u64;
	let tss_value = (limit & 0xffff)
		| ((base & 0xffffff) << 16)
		| (flags << 40)
		| (((limit >> 16) & 0x0f) << 48)
		| (((base >> 24) & 0xff) << 56);

	unsafe {
		*tss_ptr = tss_value;
	}
}

/// x86. Updates the TSS into the GDT.
#[inline(always)]
pub fn flush() {
	unsafe {
		tss_flush();
	}
}

/// Returns a reference to the TSS structure.
#[inline(always)]
pub fn get() -> &'static mut TSSEntry {
	unsafe {
		&mut *tss_get()
	}
}
