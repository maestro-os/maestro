//! The truncate syscall allows to truncate a file.

use crate::errno::Errno;
use crate::file::path::Path;
use crate::file::vfs;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn truncate(path: SyscallString, length: usize) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let mem_space_mutex = proc.get_mem_space().unwrap();
	let mem_space = mem_space_mutex.lock();

	let path = Path::from_str(path.get(&mem_space)?.ok_or(errno!(EFAULT))?, true)?;
	let path = super::util::get_absolute_path(&proc, path)?;

	let vfs_mutex = vfs::get();
	let mut vfs = vfs_mutex.lock();
	let vfs = vfs.as_mut().unwrap();

	let file_mutex = vfs.get_file_from_path(&path, proc.euid, proc.egid, true)?;
	let mut file = file_mutex.lock();

	file.set_size(length as _);

	Ok(0)
}
