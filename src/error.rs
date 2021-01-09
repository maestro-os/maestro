/*
 * This file handles software and hardware error interruptions.
 *
 * TODO doc
 */

// TODO Add non-error interrupts?

use crate::idt;
use crate::util::container::Box;
use crate::util::container::Vec;
use crate::util;

// TODO Arch dependent
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
 * Trait representing a callback that aims to be called whenever an associated interruption is
 * triggered.
 */
pub trait InterruptCallback {
	/*
	 * Tells whether the callback is enabled or not.
	 */
	fn is_enabled(&self) -> bool;

	/*
	 * Calls the callback.
	 * `id` is the id of the interrupt.
	 * `code` is an optional code associated with the interrupt. If no code is given, the value is
	 * `0`.
	 * `regs` the values of the registers when the interruption was triggered.
	 */
	fn call(&mut self, id: u32, code: u32, regs: &util::Regs);
}

/*
 * Structure wrapping a callback to insert it into a linked list.
 */
struct CallbackWrapper {
	/* The priority associated with the callback. Higher value means higher priority */
	priority: u32,
	/* The callback */
	callback: Box::<dyn InterruptCallback>,
}

// TODO Wrap in mutex
/*
 * List containing vectors that store callbacks for every interrupt watchdogs.
 */
static mut CALLBACKS: [Option::<Vec::<CallbackWrapper>>; idt::ENTRIES_COUNT as _]
	= [None; idt::ENTRIES_COUNT as _];

/*
 * Registers the given callback and returns a reference to it.
 * `id` is the id of the interrupt to watch.
 * `priority` is the priority for the callback. Higher value means higher priority.
 * `callback` is the callback to register.
 *
 * If the `id` is invalid or if an allocation fails, the function shall return an error.
 */
// TODO Return a reference?
pub fn register_callback<T: 'static + InterruptCallback>(id: u8, priority: u32, callback: T)
	-> Result<(), ()> {
	if id >= idt::ENTRIES_COUNT {
		return Err(());
	}

	let vec = unsafe { // Access to global variable
		&mut CALLBACKS[id as usize]
	};
	if vec.is_none() {
		*vec = Some(Vec::<CallbackWrapper>::new());
	}

	vec.as_mut().unwrap().push(CallbackWrapper {
		priority: priority,
		callback: Box::new(callback)?,
	});
	Ok(())
}

// TODO Callback unregister

/*
 * This function is called whenever an error interruption is triggered.
 * TODO doc
 */
#[no_mangle]
pub extern "C" fn error_handler(error: u32, error_code: u32, _regs: *const util::Regs) {
	// TODO Allow to register error callbacks
	crate::kernel_panic!(get_error_message(error), error_code);
}
