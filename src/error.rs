/*
 * This file handles software and hardware error interruptions.
 *
 * TODO doc
 */

// TODO Add non-error interrupts?

use crate::util;

/*
 * The list of interrupt error messages ordered by index of the corresponding interrupt vector.
 */
static ERROR_MESSAGES: &'static [&'static str] = &[
	"Divide-by-zero Error",
	"Debug",
	"Non-maskable Interrupt",
	"Breakpoint",
	"Overflow",
	"Bound Range Exceeded",
	"Invalid Opcode",
	"Device Not Available",
	"Double Fault",
	"Coprocessor Segment Overrun",
	"Invalid TSS",
	"Segment Not Present",
	"Stack-Segment Fault",
	"General Protection Fault",
	"Page Fault",
	"Unknown",
	"x87 Floating-Point Exception",
	"Alignement Check",
	"Machine Check",
	"SIMD Floating-Point Exception",
	"Virtualization Exception",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Security Exception",
	"Unknown"
];

/*
 * Returns the error message corresponding to the given interrupt vector index `i`.
 */
fn get_error_message(i: u32) -> &'static str {
	debug_assert!((i as usize) < ERROR_MESSAGES.len());
	ERROR_MESSAGES[i as usize]
}

/*
 * This function is called whenever an error interruption is triggered.
 * TODO doc
 */
#[no_mangle]
pub extern "C" fn error_handler(error: u32, error_code: u32, _regs: *const util::Regs) {
	// TODO Allow to register error callbacks
	::kernel_panic!(get_error_message(error), error_code);
}
