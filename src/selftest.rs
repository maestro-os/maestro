//! This module handles selftesting of the kernel. A selftest can be either a unit test or an
//! integration test.
//! The kernel uses the serial communication interface to transmit the results of the selftests to
//! another machine.

//use crate::device::serial;
use core::any::type_name;

/// Boolean value telling whether selftesting is running.
static mut RUNNING: bool = false;

/// This module contains utilities to manipulate QEMU for testing.
#[cfg(config_debug_qemu)]
pub mod qemu {
	use crate::io;

	/// The port used to trigger QEMU emulator exit with the given exit code.
	const EXIT_PORT: u16 = 0xf4;

	/// QEMU exit code for success.
	pub const SUCCESS: u32 = 0x10;
	/// QEMU exit code for failure.
	pub const FAILURE: u32 = 0x11;

	/// Exits QEMU with the given status.
	pub fn exit(status: u32) -> ! {
		unsafe {
			io::outl(EXIT_PORT, status);
		}

		crate::halt();
	}
}

/// Trait for any testable feature.
pub trait Testable {
	/// Function called to run the corresponding test.
	fn run(&self);
}

impl<T> Testable for T where T: Fn() {
	// TODO Use a special format on serial to be parsed by host?
	fn run(&self) {
		//let serial_guard = serial::get(serial::COM1).lock();

		let name = type_name::<T>();
		crate::print!("test {} ... ", name);

		self();

		//let status = "ok"; // TODO On panic, retrieve message and print on serial
		//if let Some(s) = serial {
		//	// TODO Add an additional message on fail
		//	s.write(b"{\"name\": \"");
		//	s.write(name.as_bytes());
		//	s.write(b"\", \"status\": \"");
		//	s.write(status.as_bytes());
		//	s.write(b"\"}\n");
		//}

		crate::println!("ok");
	}
}

/// The test runner for the kernel. This function runs every tests for the kernel and halts the
/// kernel or exits the emulator if possible.
pub fn runner(tests: &[&dyn Testable]) {
	crate::println!("Running {} tests", tests.len());

	unsafe { // Safe because the function is called by only one thread
		RUNNING = true;
	}

	for test in tests {
		test.run();
	}

	unsafe { // Safe because the function is called by only one thread
		RUNNING = false;
	}

	crate::println!("No more tests to run");

	#[cfg(config_debug_qemu)]
	qemu::exit(qemu::SUCCESS);
	#[cfg(not(config_debug_qemu))]
	crate::halt();
}

/// Tells whether selftesting is running.
pub fn is_running() -> bool {
	unsafe { // Safe because the function is called by only one thread
		RUNNING
	}
}
