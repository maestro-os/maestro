/// This module handles selftesting of the kernel.

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
		unsafe { // Call to unsafe functions
			io::outl(EXIT_PORT, status);
			crate::kernel_halt();
		}
	}
}

/// Trait for any testable feature.
pub trait Testable {
	/// Function called to run the corresponding test.
	fn run(&self);
}

impl<T> Testable for T where T: Fn() {
	fn run(&self) {
		crate::print!("test {} ... ", core::any::type_name::<T>());
		self();
		crate::println!("ok");
	}
}

/// The test runner for the kernel. This function runs every tests for the kernel and halts the
/// kernel or exits the emulator if possible.
pub fn runner(tests: &[&dyn Testable]) {
	crate::println!("Running {} tests", tests.len());

	for test in tests {
		test.run();
	}

	crate::println!("No more tests to run");

	#[cfg(config_debug_qemu)]
	qemu::exit(qemu::SUCCESS); // TODO Handle assertion fail (exit with FAILURE)
	unsafe { // Call to unsafe function
		crate::kernel_halt();
	}
}
