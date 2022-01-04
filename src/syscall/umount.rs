//! The `umount` system call allows to unmount a filesystem previously mounted with `mount`.

use crate::errno::Errno;
use crate::errno;
use crate::file::mountpoint;
use crate::file::path::Path;
use crate::process::Process;
use crate::process::regs::Regs;

/// The implementation of the `umount` syscall.
pub fn umount(regs: &Regs) -> Result<i32, Errno> {
	let target = regs.ebx as *const u8;

	// Getting a slice to the string
	let target_slice = {
		// Getting the process
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get();

		super::util::get_str(proc, target)?
	};

	// Getting the mountpoint
	let target_path = Path::from_str(target_slice, true)?;
	let _mountpoint = mountpoint::from_path(&target_path).ok_or(errno::EINVAL)?;

	// TODO Check if busy (EBUSY)
	// TODO If not, unmount

	Ok(0)
}
