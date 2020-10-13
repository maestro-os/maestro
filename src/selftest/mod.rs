/*
 * TODO doc
 */

// TODO Add flag to enable/disable qemu
pub mod qemu {
	use crate::io;

	/*
	 * The port used to trigger QEMU emulator exit with the given exit code.
	 */
	const EXIT_PORT: u16 = 0xf4;

	/*
	 * QEMU exit code for success.
	 */
	pub const SUCCESS: u32 = 0x10;
	/*
	 * QEMU exit code for failure.
	 */
	pub const FAILURE: u32 = 0x11;

	pub fn exit(status: u32) -> ! {
		unsafe {
			io::outl(EXIT_PORT, status);
			crate::kernel_halt();
		}
	}
}

/*
 * The test runner for the kernel. This function runs every tests for the kernel and halts the
 * kernel or exits the emulator if possible.
 */
#[cfg(test)]
pub fn runner(tests: &[&dyn Fn()]) {
    ::println!("Running {} tests", tests.len());

    for test in tests {
		// TODO Print test name
        test();
		::println!("ok");
    }

	::println!("No more tests to run");

	// TODO Add flag to enable/disable qemu
	//qemu::exit(qemu::SUCCESS);
	unsafe {
		crate::kernel_halt();
	}
}
