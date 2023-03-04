//! The `fstat64` system call allows get the status of a file.

use core::ffi::c_int;
use core::ffi::c_long;
use crate::errno::Errno;
use crate::file::Gid;
use crate::file::INode;
use crate::file::Mode;
use crate::file::Uid;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::time::unit::TimeUnit;
use crate::time::unit::Timespec;
use crate::time::unit::TimestampScale;
use crate::util::io::IO;
use macros::syscall;

// TODO Check types
/// Structure containing the informations of a file.
#[repr(C)]
#[derive(Debug)]
struct Stat {
	/// TODO doc
	st_dev: u64,

	/// Padding.
	__st_dev_padding: c_int,

	/// The inode number.
	st_ino: INode,
	/// File's mode.
	st_mode: Mode,
	/// Number of hard links to the file.
	st_nlink: u32,
	/// File's owner UID.
	st_uid: Uid,
	/// File's owner GID.
	st_gid: Gid,
	/// TODO doc
	st_rdev: u64,

	/// Padding.
	__st_rdev_padding: c_int,

	/// TODO doc
	st_size: u32,
	/// TODO doc
	st_blksize: c_long,
	/// TODO doc
	st_blocks: u64,

	/// TODO doc
	st_atim: Timespec,
	/// TODO doc
	st_mtim: Timespec,
	/// TODO doc
	st_ctim: Timespec,
}

#[syscall]
pub fn fstat64(fd: c_int, statbuf: SyscallPtr<Stat>) -> Result<i32, Errno> {
	if fd < 0 {
		return Err(errno!(EBADF));
	}

	let open_file_mutex = {
		let proc_mutex = Process::get_current().unwrap();
		let proc_guard = proc_mutex.lock();
		let proc = proc_guard.get_mut();

		let fds_mutex = proc.get_fds().unwrap();
		let fds_guard = fds_mutex.lock();
		let fds = fds_guard.get();

		fds.get_fd(fd as _)
			.ok_or_else(|| errno!(EBADF))?
			.get_open_file()?
	};
	let open_file_guard = open_file_mutex.lock();
	let open_file = open_file_guard.get();

	let file_mutex = open_file.get_file()?;
	let file_guard = file_mutex.lock();
	let file = file_guard.get();

	let inode = file.get_location().get_inode();

	let stat = Stat {
		st_dev: 0, // TODO

		__st_dev_padding: 0,

		st_ino: inode,
		st_mode: file.get_mode(),
		st_nlink: file.get_hard_links_count() as _,
		st_uid: file.get_uid(),
		st_gid: file.get_gid(),
		st_rdev: 0, // TODO

		__st_rdev_padding: 0,

		st_size: file.get_size() as _,
		st_blksize: 512, // TODO
		st_blocks: file.get_blocks_count(),

		st_atim: Timespec::from_nano(TimestampScale::convert(
			file.get_atime(),
			TimestampScale::Second,
			TimestampScale::Nanosecond
		)),
		st_mtim: Timespec::from_nano(TimestampScale::convert(
			file.get_mtime(),
			TimestampScale::Second,
			TimestampScale::Nanosecond
		)),
		st_ctim: Timespec::from_nano(TimestampScale::convert(
			file.get_ctime(),
			TimestampScale::Second,
			TimestampScale::Nanosecond
		)),
	};

	{
		let proc_mutex = Process::get_current().unwrap();
		let proc_guard = proc_mutex.lock();
		let proc = proc_guard.get_mut();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let statbuf = statbuf.get_mut(&mem_space_guard)?.ok_or(errno!(EFAULT))?;
		*statbuf = stat;
	}

	Ok(0)
}
