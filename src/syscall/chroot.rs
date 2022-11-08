//! The `chroot` system call allows to virtually redefine the system's root for
//! the current process.

use crate::errno::Errno;
use crate::file;
use crate::file::path::Path;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;
use crate::process::Process;
use crate::vfs;

/// The implementation of the `chroot` syscall.
pub fn chroot(regs: &Regs) -> Result<i32, Errno> {
	let path: SyscallString = (regs.ebx as usize).into();

	let proc_mutex = Process::get_current().unwrap();
	let proc_guard = proc_mutex.lock();
	let proc = proc_guard.get_mut();

	let uid = proc.get_euid();
	let gid = proc.get_egid();

	// Checking permission
	if uid != file::ROOT_UID {
		return Err(errno!(EPERM));
	}

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();
	let path = path.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;
	let path = Path::from_str(path, true)?;

	// Checking access to file
	let vfs_mutex = vfs::get();
	let vfs_guard = vfs_mutex.lock();
	let vfs = vfs_guard.get_mut().as_mut().unwrap();
	vfs.get_file_from_path(&path, uid, gid, true)?;

	proc.set_chroot(path);
	Ok(0)
}
