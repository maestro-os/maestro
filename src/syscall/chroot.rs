//! The `chroot` system call allows to virtually redefine the system's root for
//! the current process.

use crate::errno::Errno;
use crate::file;
use crate::file::path::Path;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use crate::vfs;
use macros::syscall;

#[syscall]
pub fn chroot(path: SyscallString) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let mut proc = proc_mutex.lock();

	let uid = proc.euid;
	let gid = proc.egid;

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
	let mut vfs = vfs_mutex.lock();
	let vfs = vfs.as_mut().unwrap();
	vfs.get_file_from_path(&path, uid, gid, true)?;

	proc.chroot = path;
	Ok(0)
}
