//! The `chown` system call changes the owner of a file.

use crate::errno::EResult;
use crate::errno::Errno;
use crate::file::path::PathBuf;
use crate::file::vfs;
use crate::file::vfs::ResolutionSettings;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use core::ffi::c_int;
use macros::syscall;

/// Performs the `chown` syscall.
pub fn do_chown(
	pathname: SyscallString,
	owner: c_int,
	group: c_int,
	follow_links: bool,
) -> EResult<i32> {
	if owner < -1 || group < -1 {
		return Err(errno!(EINVAL));
	}

	let (path, rs) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space = mem_space.lock();

		let path = pathname.get(&*mem_space)?.ok_or_else(|| errno!(EFAULT))?;
		let path = PathBuf::try_from(path)?;

		let rs = ResolutionSettings::for_process(&proc, follow_links);
		(path, rs)
	};

	let file_mutex = vfs::get_file_from_path(&path, &rs)?;
	let mut file = file_mutex.lock();
	// TODO allow changing group to any group whose owner is member
	if !rs.access_profile.is_privileged() {
		return Err(errno!(EPERM));
	}
	if owner != -1 {
		file.set_uid(owner as _);
	}
	if group != -1 {
		file.set_gid(group as _);
	}
	// TODO lazy
	file.sync()?;

	Ok(0)
}

#[syscall]
pub fn chown(pathname: SyscallString, owner: c_int, group: c_int) -> EResult<i32> {
	do_chown(pathname, owner, group, true)
}
