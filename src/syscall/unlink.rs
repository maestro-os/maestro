//! The `unlink` system call deletes the given link from its filesystem.
//!
//! If no link remain to the file, the function also removes it.

use crate::errno::Errno;
use crate::file::path::Path;
use crate::file::vfs;
use crate::file::vfs::ResolutionSettings;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn unlink(pathname: SyscallString) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let mem_space_mutex = proc.get_mem_space().unwrap();
	let mem_space = mem_space_mutex.lock();
	let path = pathname.get(&mem_space)?.ok_or(errno!(EFAULT))?;
	let path = Path::new(path)?;

	let rs = ResolutionSettings::for_process(&proc, true);

	// Remove the file
	vfs::remove_file_from_path(path, &rs)?;

	Ok(0)
}
