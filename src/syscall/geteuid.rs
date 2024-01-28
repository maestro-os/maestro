//! The `geteuid` syscall returns the effective UID of the process's owner.

use crate::{errno::Errno, process::Process};
use macros::syscall;

#[syscall]
pub fn geteuid() -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();
	Ok(proc.access_profile.get_euid() as _)
}
