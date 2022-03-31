//! The uname syscall is used to retrieve informations about the system.

use core::cmp::min;
use core::mem::size_of;
use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::Regs;

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

/// Copies from slice `src` to `dst`. If one slice is smaller than the other, the function stops
/// when the end of the smallest is reached.
fn slice_copy(src: &[u8], dst: &mut [u8]) {
	let len = min(src.len(), dst.len());
	dst[..len].copy_from_slice(&src[..len]);
}

/// The implementation of the `uname` syscall.
pub fn uname(regs: &Regs) -> Result<i32, Errno> {
	let buf = regs.ebx as *mut Utsname;

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();
	if !proc.get_mem_space().unwrap().can_access(buf as _, size_of::<Utsname>(), true, true) {
		return Err(errno!(EFAULT));
	}

	let utsname = unsafe {
		&mut *buf
	};
	*utsname = Utsname {
		sysname: [0; UTSNAME_LENGTH],
		nodename: [0; UTSNAME_LENGTH],
		release: [0; UTSNAME_LENGTH],
		version: [0; UTSNAME_LENGTH],
		machine: [0; UTSNAME_LENGTH],
	};

	slice_copy(&crate::NAME.as_bytes(), &mut utsname.sysname);
	slice_copy(&[], &mut utsname.nodename);
	slice_copy(&crate::VERSION.as_bytes(), &mut utsname.release);
	slice_copy(&[], &mut utsname.version);
	slice_copy(&"x86".as_bytes(), &mut utsname.machine); // TODO Adapt to current architecture

	Ok(0)
}
