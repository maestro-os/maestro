//! The `access` system call allows to check access to a given file.

use crate::errno::Errno;
use crate::file::path::Path;
use crate::file::vfs;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;
use crate::process::Process;
use crate::util::FailableClone;

/// Special value, telling to take the path relative to the current working
/// directory.
pub const AT_FDCWD: i32 = -100;
/// If pathname is a symbolic link, do not dereference it: instead return
/// information about the link itself.
pub const AT_SYMLINK_NOFOLLOW: i32 = 0x100;
/// Perform access checks using the effective user and group IDs.
pub const AT_EACCESS: i32 = 0x200;
/// TODO doc
pub const AT_NO_AUTOMOUNT: i32 = 0x800;
/// TODO doc
pub const AT_EMPTY_PATH: i32 = 0x1000;
/// TODO doc
pub const AT_STATX_SYNC_AS_STAT: i32 = 0x0000;
/// TODO doc
pub const AT_STATX_FORCE_SYNC: i32 = 0x2000;
/// TODO doc
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
/// `dirfd` is the file descriptor of the directory relative to which the check
/// is done. `pathname` is the path to the file.
/// `mode` is a bitfield of access permissions to check.
/// `flags` is a set of flags.
pub fn do_access(
	dirfd: Option<i32>,
	pathname: SyscallString,
	mode: i32,
	flags: Option<i32>,
) -> Result<i32, Errno> {
	let flags = flags.unwrap_or(0);

	let follow_symlinks = flags & AT_SYMLINK_NOFOLLOW == 0;

	let (path, uid, gid, cwd) = {
		let proc_mutex = Process::get_current().unwrap();
		let proc_guard = proc_mutex.lock();
		let proc = proc_guard.get();

		let mem_space_mutex = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space_mutex.lock();

		let pathname = pathname
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EINVAL))?;
		let path = Path::from_str(pathname, true)?;

		let (uid, gid) = {
			if flags & AT_EACCESS != 0 {
				(proc.get_euid(), proc.get_egid())
			} else {
				(proc.get_uid(), proc.get_gid())
			}
		};

		let cwd = proc.get_cwd().failable_clone()?;
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

		let vfs_guard = vfs::get().lock();
		let vfs = vfs_guard.get_mut().as_mut().unwrap();
		vfs.get_file_from_path(&path, uid, gid, follow_symlinks)?
	};

	{
		let file_guard = file.lock();
		let file = file_guard.get();

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

	return Ok(0);
}

/// The implementation of the `access` syscall.
pub fn access(regs: &Regs) -> Result<i32, Errno> {
	let pathname: SyscallString = (regs.ebx as usize).into();
	let mode = regs.ecx as i32;

	do_access(None, pathname, mode, None)
}
