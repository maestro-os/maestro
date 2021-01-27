/// This file handles interruptions, it provides an interface allowing to register callbacks for
/// each interrupts. Each callback has a priority number and is called in descreasing order.

use core::cmp::Ordering;
use crate::idt;
use crate::util::container::*;
use crate::util::lock::{Mutex, MutexGuard};
use crate::util;

// TODO Arch dependent
/// The list of interrupt error messages ordered by index of the corresponding interrupt vector.
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

/// Returns the error message corresponding to the given interrupt vector index `i`.
fn get_error_message(i: u32) -> &'static str {
	debug_assert!((i as usize) < ERROR_MESSAGES.len());
	ERROR_MESSAGES[i as usize]
}

/// Trait representing a callback that aims to be called whenever an associated interruption is
/// triggered.
pub trait InterruptCallback {
	/// Tells whether the callback is enabled or not.
	fn is_enabled(&self) -> bool;

	/// Calls the callback.
	/// `id` is the id of the interrupt.
	/// `code` is an optional code associated with the interrupt. If no code is given, the value is
	/// `0`.
	/// `regs` the values of the registers when the interruption was triggered.
	fn call(&self, id: u32, code: u32, regs: &util::Regs);
}

/// Structure wrapping a callback to insert it into a linked list.
struct CallbackWrapper {
	/// The priority associated with the callback. Higher value means higher priority 
	priority: u32,
	/// The callback 
	callback: Box::<dyn InterruptCallback>,
}

/// List containing vectors that store callbacks for every interrupt watchdogs.
static mut CALLBACKS: Mutex::<[Option::<Vec::<CallbackWrapper>>; idt::ENTRIES_COUNT as _]>
	= Mutex::new([None; idt::ENTRIES_COUNT as _]);

/// Registers the given callback and returns a reference to it.
/// `id` is the id of the interrupt to watch.
/// `priority` is the priority for the callback. Higher value means higher priority.
/// `callback` is the callback to register.
/// 
/// If the `id` is invalid or if an allocation fails, the function shall return an error.
// TODO Return a reference?
pub fn register_callback<T: 'static + InterruptCallback>(id: u8, priority: u32, callback: T)
	-> Result<(), ()> {
	if id >= idt::ENTRIES_COUNT {
		return Err(());
	}

	let mut guard = unsafe { // Access to global variable
		MutexGuard::new(&mut CALLBACKS)
	};
	let vec = &mut guard.get_mut()[id as usize];
	if vec.is_none() {
		*vec = Some(Vec::<CallbackWrapper>::new());
	}

	let index = {
		let r = vec.as_mut().unwrap().binary_search_by(| x | {
			if x.priority < priority {
				Ordering::Less
			} else if x.priority > priority {
				Ordering::Greater
			} else {
				Ordering::Equal
			}
		});

		if let Err(l) = r {
			l
		} else {
			r.unwrap()
		}
	};
	vec.as_mut().unwrap().insert(index, CallbackWrapper {
		priority: priority,
		callback: Box::new(callback)?,
	});
	Ok(())
}

// TODO Callback unregister

/// This function is called whenever an interruption is triggered.
/// `id` is the identifier of the interrupt type. This value is architecture-dependent.
/// `code` is an optional code associated with the interrupt. If the interrupt type doesn't have a
/// code, the value is `0`.
/// `regs` is the state of the registers at the moment of the interrupt.
#[no_mangle]
pub extern "C" fn event_handler(id: u32, code: u32, regs: &util::Regs) {
	let guard = unsafe { // Access to global variable
		MutexGuard::new(&mut CALLBACKS)
	};
	let callbacks = &guard.get()[id as usize];

	if let Some(callbacks) = callbacks {
		for c in callbacks.into_iter() {
			(*c.callback).call(id, code, regs);
		}
	} else {
		crate::kernel_panic!(get_error_message(id), code);
	}
}
