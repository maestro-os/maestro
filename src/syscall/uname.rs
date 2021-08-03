//! The uname syscall is used to retrieve informations about the system.

use core::mem::size_of;
use core::ptr::copy_nonoverlapping;
use crate::errno::Errno;
use crate::errno;
use crate::kern;
use crate::process::Process;
use crate::util;

/// The length of a field of the utsname structure.
const UTSNAME_LENGTH: usize = 256;

struct Utsname {
	/// Operating system name.
	sysname: [u8; UTSNAME_LENGTH],
	/// Network node hostname.
	nodename: [u8; UTSNAME_LENGTH],
	/// Operating system release.
	release: [u8; UTSNAME_LENGTH],
	/// Operating system version.
	version: [u8; UTSNAME_LENGTH],
	/// Hardware identifier.
	machine: [u8; UTSNAME_LENGTH],
}

/// The implementation of the `uname` syscall.
pub fn uname(regs: &util::Regs) -> Result<i32, Errno> {
	let buf = regs.ebx as *mut Utsname;
	let mut utsname = Utsname {
		sysname: [0; UTSNAME_LENGTH],
		nodename: [0; UTSNAME_LENGTH],
		release: [0; UTSNAME_LENGTH],
		version: [0; UTSNAME_LENGTH],
		machine: [0; UTSNAME_LENGTH],
	};

	utsname.sysname.clone_from_slice(&kern::NAME.as_bytes());
	// TODO nodename
	utsname.release.clone_from_slice(&kern::VERSION.as_bytes());
	// TODO version
	// TODO machine

	let mut mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock(false);
	let proc = guard.get_mut();

	if proc.get_mem_space().can_access(buf as _, size_of::<Utsname>(), true, true) {
		unsafe {
			copy_nonoverlapping(&utsname as *const Utsname, buf, 1);
		}

		Ok(0)
	} else {
		Err(errno::EFAULT)
	}
}
