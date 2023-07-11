//! The uname syscall is used to retrieve informations about the system.

use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::Process;
use crate::util;
use macros::syscall;

/// The length of a field of the utsname structure.
const UTSNAME_LENGTH: usize = 65;

/// Userspace structure storing uname informations.
#[repr(C)]
#[derive(Debug)]
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

#[syscall]
pub fn uname(buf: SyscallPtr<Utsname>) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let mem_space = proc.get_mem_space().unwrap();
	let mut mem_space_guard = mem_space.lock();
	let utsname = buf.get_mut(&mut mem_space_guard)?.ok_or(errno!(EFAULT))?;

	*utsname = Utsname {
		sysname: [0; UTSNAME_LENGTH],
		nodename: [0; UTSNAME_LENGTH],
		release: [0; UTSNAME_LENGTH],
		version: [0; UTSNAME_LENGTH],
		machine: [0; UTSNAME_LENGTH],
	};

	util::slice_copy(crate::NAME.as_bytes(), &mut utsname.sysname);

	let hostname = crate::HOSTNAME.lock();
	util::slice_copy(&hostname, &mut utsname.nodename);

	util::slice_copy(crate::VERSION.as_bytes(), &mut utsname.release);
	util::slice_copy(&[], &mut utsname.version);
	util::slice_copy(crate::ARCH.as_bytes(), &mut utsname.machine);

	Ok(0)
}
