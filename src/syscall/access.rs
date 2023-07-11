//! The `access` system call allows to check access to a given file.

use crate::errno::Errno;
use crate::file::path::Path;
use crate::file::vfs;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use crate::util::TryClone;
use core::ffi::c_int;
use macros::syscall;

/// Special value, telling to take the path relative to the current working
/// directory.
pub const AT_FDCWD: i32 = -100;
/// If pathname is a symbolic link, do not dereference it: instead return
/// information about the link itself.
pub const AT_SYMLINK_NOFOLLOW: i32 = 0x100;
/// Perform access checks using the effective user and group IDs.
pub const AT_EACCESS: i32 = 0x200;
/// Don't automount the terminal component of `pathname` if it is a directory that is an automount
/// point.
pub const AT_NO_AUTOMOUNT: i32 = 0x800;
/// If `pathname` is an empty string, operate on the file referred to by `dirfd`.
pub const AT_EMPTY_PATH: i32 = 0x1000;
/// Do whatever `stat` does.
pub const AT_STATX_SYNC_AS_STAT: i32 = 0x0000;
/// Force the attributes to be synchronized with the server.
pub const AT_STATX_FORCE_SYNC: i32 = 0x2000;
/// Don't synchronize anything, but rather take cached informations.
pub const AT_STATX_DONT_SYNC: i32 = 0x4000;

/// Checks for existence of the file.
const F_OK: i32 = 0;
/// Checks the file can be read.
const R_OK: i32 = 4;
/// Checks the file can be written.
const W_OK: i32 = 2;
/// Checks the file can be executed.
const X_OK: i32 = 1;

/// Performs the access operation.
///
/// Arguments:
/// - `dirfd` is the file descriptor of the directory relative to which the check
/// is done.
/// - `pathname` is the path to the file.
/// - `mode` is a bitfield of access permissions to check.
/// - `flags` is a set of flags.
pub fn do_access(
	dirfd: Option<i32>,
	pathname: SyscallString,
	mode: i32,
	flags: Option<i32>,
) -> Result<i32, Errno> {
	let flags = flags.unwrap_or(0);

	let follow_symlinks = flags & AT_SYMLINK_NOFOLLOW == 0;

	let (path, uid, gid, cwd) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space_mutex = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space_mutex.lock();

		let pathname = pathname
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EINVAL))?;
		let path = Path::from_str(pathname, true)?;

		let (uid, gid) = {
			if flags & AT_EACCESS != 0 {
				(proc.euid, proc.egid)
			} else {
				(proc.uid, proc.gid)
			}
		};

		let cwd = proc.get_cwd().try_clone()?;
		(path, uid, gid, cwd)
	};

	// Getting file
	let file = {
		let mut path = path;

		if path.is_absolute() {
		} else if let Some(dirfd) = dirfd {
			if dirfd == AT_FDCWD {
				path = cwd.concat(&path)?;
			} else {
				// TODO Get file from fd and get its path to concat
				todo!();
			}
		}

		let mut vfs = vfs::get().lock();
		let vfs = vfs.as_mut().unwrap();
		vfs.get_file_from_path(&path, uid, gid, follow_symlinks)?
	};

	{
		let file = file.lock();

		// Do access checks
		if (mode & R_OK != 0) && !file.can_read(uid, gid) {
			return Err(errno!(EACCES));
		}
		if (mode & W_OK != 0) && !file.can_write(uid, gid) {
			return Err(errno!(EACCES));
		}
		if (mode & X_OK != 0) && !file.can_execute(uid, gid) {
			return Err(errno!(EACCES));
		}
	}

	Ok(0)
}

#[syscall]
pub fn access(pathname: SyscallString, mode: c_int) -> Result<i32, Errno> {
	do_access(None, pathname, mode, None)
}
