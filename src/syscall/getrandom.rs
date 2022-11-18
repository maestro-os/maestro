//! The `getrandom` system call allows to get random bytes.

use core::ffi::c_uint;
use crate::crypto::rand;
use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::Process;
use macros::syscall;

/// If set, bytes are draw from the random source instead of urandom.
const GRND_RANDOM: u32 = 2;
/// If set, the function doesn't block. If no entropy is available, the function
/// returns EAGAIN.
const GRND_NONBLOCK: u32 = 1;

/// Implementation of the `getrandom` syscall.
#[syscall]
pub fn getrandom(buf: SyscallSlice::<u8>, buflen: usize, flags: c_uint) -> Result<i32, Errno> {
	// Getting randomness source
	let random_source_mutex = match flags & GRND_RANDOM != 0 {
		// Using random
		true => rand::get_source("random"),

		// Using urandom
		false => rand::get_source("urandom"),
	};
	let random_source_guard = random_source_mutex
		.as_ref()
		.ok_or_else(|| errno!(EAGAIN))?
		.lock();
	let random_source = random_source_guard.get_mut();

	let nonblock = flags & GRND_NONBLOCK != 0;
	if nonblock && buflen > random_source.available_bytes() {
		return Err(errno!(EAGAIN));
	}

	// Getting current process
	let proc_mutex = Process::get_current().unwrap();
	let proc_guard = proc_mutex.lock();
	let proc = proc_guard.get();

	let mem_space_mutex = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space_mutex.lock();

	if let Some(buf) = buf.get_mut(&mem_space_guard, buflen)? {
		let mut i = 0;
		while i < buf.len() {
			i += random_source.consume_entropy(&mut buf[i..]);
		}

		Ok(buf.len() as _)
	} else {
		Ok(0)
	}
}
