/// This module contains the custom test framework for userspace testing.

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

/// The test runner. Runs every tests and exits.
pub fn runner(tests: &[&dyn Testable]) {
	crate::println!("Running {} tests", tests.len());

	for test in tests {
		test.run();
	}

	crate::println!("No more tests to run");

	#[cfg(userspace)]
	unsafe { // Call to C function
		exit(0); // TODO Handle assertion fail (exit with 1)
	}
}
