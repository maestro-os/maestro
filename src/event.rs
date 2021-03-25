/// This file handles interruptions, it provides an interface allowing to register callbacks for
/// each interrupts. Each callback has a priority number and is called in descreasing order.

use core::cmp::Ordering;
use core::mem::MaybeUninit;
use crate::errno::Errno;
use crate::idt::pic;
use crate::idt;
use crate::process::tss;
use crate::util::container::vec::Vec;
use crate::util::lock::mutex::{Mutex, MutexGuard};
use crate::util::ptr::SharedPtr;
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
	if (i as usize) < ERROR_MESSAGES.len() {
		ERROR_MESSAGES[i as usize]
	} else {
		"Unknown"
	}
}

/// The action to execute after the interrupt handler has returned.
pub enum InterruptResultAction {
	/// Resumes execution of the code where it was interrupted.
	Resume,
	/// Goes back to the kernel loop, waiting for another interruption.
	Loop,
	/// Makes the kernel panic.
	Panic,
}

/// Enumeration telling which action will be executed after an interrupt handler.
pub struct InterruptResult {
	/// Tells whether to skip execution of the next interrupt handlers (with lower priority).
	skip_next: bool,
	/// The action to execute after the handler. The last handler decides which action to execute
	/// unless the `skip_next` variable is set to `true`.
	action: InterruptResultAction,
}

impl InterruptResult {
	/// Creates a new instance.
	pub fn new(skip_next: bool, action: InterruptResultAction) -> Self {
		Self {
			skip_next: skip_next,
			action: action,
		}
	}
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
	/// If the function returns `false`, the kernel shall panic.
	fn call(&mut self, id: u32, code: u32, regs: &util::Regs) -> InterruptResult;
}

/// Structure wrapping a callback to insert it into a linked list.
struct CallbackWrapper {
	/// The priority associated with the callback. Higher value means higher priority
	priority: u32,
	/// The callback
	callback: SharedPtr::<dyn InterruptCallback>,
}

/// List containing vectors that store callbacks for every interrupt watchdogs.
static mut CALLBACKS: MaybeUninit::<
		Mutex::<[Option::<Vec::<CallbackWrapper>>; idt::ENTRIES_COUNT as _]>
	> = MaybeUninit::uninit();

/// Initializes the events handler.
pub fn init() {
	let mut guard = MutexGuard::new(unsafe { // Access to global variable
		CALLBACKS.assume_init_mut()
	});
	let callbacks = guard.get_mut();

	for i in 0..callbacks.len() {
		callbacks[i] = None;
	}
}

/// Registers the given callback and returns a reference to it.
/// `id` is the id of the interrupt to watch.
/// `priority` is the priority for the callback. Higher value means higher priority.
/// `callback` is the callback to register.
///
/// If the `id` is invalid or if an allocation fails, the function shall return an error.
pub fn register_callback<T: 'static + InterruptCallback>(id: usize, priority: u32, callback: T)
	-> Result<SharedPtr::<T>, Errno> {
	debug_assert!(id < idt::ENTRIES_COUNT);

	let mut guard = unsafe { // Access to global variable
		MutexGuard::new(CALLBACKS.assume_init_mut())
	};
	let vec = &mut guard.get_mut()[id];
	if vec.is_none() {
		*vec = Some(Vec::<CallbackWrapper>::new());
	}
	let v = vec.as_mut().unwrap();

	let index = {
		let r = v.binary_search_by(| x | {
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

	let mut ptr = SharedPtr::new(callback)?;
	v.insert(index, CallbackWrapper {
		priority: priority,
		callback: ptr.clone(),
	})?;
	Ok(ptr)
}

// TODO Callback unregister

/// This function is called whenever an interruption is triggered.
/// `id` is the identifier of the interrupt type. This value is architecture-dependent.
/// `code` is an optional code associated with the interrupt. If the interrupt type doesn't have a
/// code, the value is `0`.
/// `regs` is the state of the registers at the moment of the interrupt.
#[no_mangle]
pub extern "C" fn event_handler(id: u32, code: u32, regs: &util::Regs) {
	let mutex = unsafe { // Access to global variable
		CALLBACKS.assume_init_mut()
	};
	if mutex.is_locked() {
		crate::kernel_panic!("Event handler deadlock");
	}
	let mut guard = MutexGuard::new(mutex);

	if let Some(callbacks) = &mut guard.get_mut()[id as usize] {
		let mut last_action = InterruptResultAction::Resume;

		for i in 0..callbacks.len() {
			if (*callbacks[i].callback).is_enabled() {
				let result = (*callbacks[i].callback).call(id, code, regs);
				last_action = result.action;
				if result.skip_next {
					break;
				}
			}
		}

		match last_action {
			InterruptResultAction::Resume => {},
			InterruptResultAction::Loop => {
				pic::end_of_interrupt(id as _);
				// TODO Fix: Use of loop action before TSS init shall result in undefined behaviour
				// TODO Fix: The stack might be removed while being used (example: process is
				// killed, its exit status is retrieved from another CPU core and then the process
				// is removed)
				unsafe { // Call to ASM function
					crate::kernel_loop_reset(tss::get().esp0 as _);
				}
			},
			InterruptResultAction::Panic => {
				crate::kernel_panic!(get_error_message(id), code);
			},
		}
	} else if (id as usize) < ERROR_MESSAGES.len() {
		crate::kernel_panic!(get_error_message(id), code);
	}
}
