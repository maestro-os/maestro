//! The `unlinkat` syscall allows to unlink a file.
//!
//! If no link remain to the file, the function also removes it.

use super::util::at;
use crate::{
	errno::Errno,
	file::{
		path::PathBuf,
		vfs,
		vfs::{ResolutionSettings, Resolved},
	},
	process::{mem_space::ptr::SyscallString, Process},
};
use core::ffi::c_int;
use macros::syscall;

#[syscall]
pub fn unlinkat(dirfd: c_int, pathname: SyscallString, flags: c_int) -> Result<i32, Errno> {
	let (fds_mutex, path, rs) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let rs = ResolutionSettings::for_process(&proc, false);

		let mem_space = proc.get_mem_space().unwrap().clone();
		let mem_space_guard = mem_space.lock();

		let fds_mutex = proc.file_descriptors.clone().unwrap();

		let pathname = pathname
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let path = PathBuf::try_from(pathname)?;

		(fds_mutex, path, rs)
	};

	let fds = fds_mutex.lock();
	let resolved = at::get_file(&fds, rs.clone(), dirfd, &path, flags)?;
	match resolved {
		Resolved::Found(parent_mutex) => {
			let mut parent = parent_mutex.lock();
			let name = path.file_name().ok_or_else(|| errno!(ENOENT))?;
			vfs::remove_file(&mut parent, name, &rs.access_profile)?;
		}
		_ => return Err(errno!(ENOENT)),
	}

	Ok(0)
}
