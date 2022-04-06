//! The uname syscall is used to retrieve informations about the system.

use crate::errno::Errno;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::regs::Regs;
use crate::util;

/// The length of a field of the utsname structure.
const UTSNAME_LENGTH: usize = 65;

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
pub fn uname(regs: &Regs) -> Result<i32, Errno> {
	let buf: SyscallPtr<Utsname> = (regs.ebx as usize).into();

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	let mem_space_guard = proc.get_mem_space().unwrap().lock();
	let utsname = buf.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;

	*utsname = Utsname {
		sysname: [0; UTSNAME_LENGTH],
		nodename: [0; UTSNAME_LENGTH],
		release: [0; UTSNAME_LENGTH],
		version: [0; UTSNAME_LENGTH],
		machine: [0; UTSNAME_LENGTH],
	};

	util::slice_copy(&crate::NAME.as_bytes(), &mut utsname.sysname);
	util::slice_copy(&[], &mut utsname.nodename);
	util::slice_copy(&crate::VERSION.as_bytes(), &mut utsname.release);
	util::slice_copy(&[], &mut utsname.version);
	util::slice_copy(&"x86".as_bytes(), &mut utsname.machine); // TODO Adapt to current architecture

	Ok(0)
}
