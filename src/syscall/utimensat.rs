//! The `utimensat` system call allows to change the timestamps of a file.

use super::access::AT_FDCWD;
use super::util;
use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use crate::time::unit::TimeUnit;
use crate::time::unit::Timespec;
use core::ffi::c_int;
use macros::syscall;

#[syscall]
pub fn utimensat(
	dirfd: c_int,
	pathname: SyscallString,
	times: SyscallPtr<[Timespec; 2]>,
	flags: c_int,
) -> Result<i32, Errno> {
	let (file_mutex, atime, mtime) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let file_mutex = match pathname.get(&mem_space_guard)? {
			Some(pathname) => util::get_file_at(proc, true, dirfd, pathname, flags)?,

			None if dirfd != AT_FDCWD => {
				if dirfd < 0 {
					return Err(errno!(EBADF));
				}

				proc.get_fds()
					.unwrap()
					.lock()
					.get_fd(dirfd as _)
					.ok_or(errno!(EBADF))?
					.get_open_file()?
					.lock()
					.get_file()?
			}

			_ => return Err(errno!(EFAULT)),
		};

		let times_val = times.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;

		let atime = times_val[0];
		let mtime = times_val[1];

		(file_mutex, atime, mtime)
	};

	let mut file = file_mutex.lock();

	// TODO clean
	file.atime = atime.to_nano() / 1000000000;
	file.mtime = mtime.to_nano() / 1000000000;

	// TODO sync only when required
	file.sync()?;

	Ok(0)
}
