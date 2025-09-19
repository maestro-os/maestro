/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! Files handling system calls.

use crate::{
	device::id,
	file,
	file::{
		File, FileType, O_CLOEXEC, O_CREAT, O_DIRECTORY, O_EXCL, O_NOCTTY, O_NOFOLLOW, O_RDONLY,
		O_RDWR, O_TRUNC, O_WRONLY, Stat,
		fd::{FD_CLOEXEC, fd_to_file},
		fs::StatSet,
		perm::AccessProfile,
		vfs,
		vfs::{ResolutionSettings, Resolved},
	},
	memory::user::{UserPtr, UserSlice, UserString},
	process::Process,
	syscall::util::{
		at,
		at::{AT_EACCESS, AT_EMPTY_PATH, AT_FDCWD, AT_SYMLINK_NOFOLLOW},
	},
	time::{
		clock::{Clock, current_time_ns, current_time_sec},
		unit::{TimeUnit, Timespec},
	},
};
use core::{ffi::c_int, hint::unlikely, sync::atomic::Ordering::Release};
use utils::{collections::path::Path, errno, errno::EResult, limits::SYMLINK_MAX, ptr::arc::Arc};

/// `access` flag: Checks for existence of the file.
const F_OK: i32 = 0;
/// `access` flag: Checks the file can be read.
const R_OK: i32 = 4;
/// `access` flag: Checks the file can be written.
const W_OK: i32 = 2;
/// `access` flag: Checks the file can be executed.
const X_OK: i32 = 1;

/// `rename` flag: Don't replace new path if it exists. Return an error instead.
const RENAME_NOREPLACE: c_int = 1;
/// `rename` flag: Exchanges old and new paths atomically.
const RENAME_EXCHANGE: c_int = 2;

pub fn creat(pathname: UserString, mode: c_int) -> EResult<usize> {
	do_openat(AT_FDCWD, pathname, O_CREAT | O_WRONLY | O_TRUNC, mode as _)
}

pub fn mkdir(pathname: UserString, mode: file::Mode) -> EResult<usize> {
	let path = pathname.copy_path_from_user()?;
	// If the path is not empty, create
	if let Some(name) = path.file_name() {
		// Get parent directory
		let parent_path = path.parent().unwrap_or(Path::root());
		let parent = vfs::get_file_from_path(parent_path, true)?;
		let mode = mode & !Process::current().umask();
		let ts = current_time_sec(Clock::Realtime);
		// Create the directory
		vfs::create_file(
			parent,
			name,
			Stat {
				mode: FileType::Directory.to_mode() | mode,
				ctime: ts,
				mtime: ts,
				atime: ts,
				..Default::default()
			},
		)?;
	}
	Ok(0)
}

pub fn mknod(pathname: UserString, mode: file::Mode, dev: u64) -> EResult<usize> {
	let pathname = pathname.copy_path_from_user()?;
	let parent_path = pathname.parent().unwrap_or(Path::root());
	// File name
	let Some(name) = pathname.file_name() else {
		return Err(errno!(EEXIST));
	};
	// Check file type and permissions
	let mode = mode & !Process::current().umask();
	let file_type = FileType::from_mode(mode).ok_or(errno!(EPERM))?;
	let privileged = AccessProfile::cur_task().is_privileged();
	match (file_type, privileged) {
		(FileType::Regular | FileType::Fifo | FileType::Socket, _) => {}
		(FileType::BlockDevice | FileType::CharDevice, true) => {}
		(_, false) => return Err(errno!(EPERM)),
		(_, true) => return Err(errno!(EINVAL)),
	}
	// Create file
	let ts = current_time_sec(Clock::Realtime);
	let parent = vfs::get_file_from_path(parent_path, true)?;
	vfs::create_file(
		parent,
		name,
		Stat {
			mode,
			dev_major: id::major(dev),
			dev_minor: id::minor(dev),
			ctime: ts,
			mtime: ts,
			atime: ts,
			..Default::default()
		},
	)?;
	Ok(0)
}

pub fn link(oldpath: UserString, newpath: UserString) -> EResult<usize> {
	linkat(AT_FDCWD, oldpath, AT_FDCWD, newpath, 0)
}

pub fn linkat(
	olddirfd: c_int,
	oldpath: UserString,
	newdirfd: c_int,
	newpath: UserString,
	flags: c_int,
) -> EResult<usize> {
	let oldpath = oldpath.copy_path_from_user()?;
	let newpath = newpath.copy_path_from_user()?;
	// Get old file
	let Resolved::Found(old) = at::get_file(olddirfd, Some(&oldpath), flags, false, false)? else {
		unreachable!();
	};
	if old.get_type()? == FileType::Directory {
		return Err(errno!(EPERM));
	}
	// Create new file
	let Resolved::Creatable {
		parent: new_parent,
		name: new_name,
	} = at::get_file(newdirfd, Some(&newpath), 0, true, true)?
	else {
		return Err(errno!(EEXIST));
	};
	let name = new_name.try_into()?;
	vfs::link(&new_parent, name, old.node().clone())?;
	Ok(0)
}

pub fn symlink(target: UserString, linkpath: UserString) -> EResult<usize> {
	symlinkat(target, AT_FDCWD, linkpath)
}

pub fn symlinkat(target: UserString, newdirfd: c_int, linkpath: UserString) -> EResult<usize> {
	let target = target.copy_path_from_user()?;
	if target.len() > SYMLINK_MAX {
		return Err(errno!(ENAMETOOLONG));
	}
	let linkpath = linkpath.copy_path_opt_from_user()?;
	// Create link
	let Resolved::Creatable {
		parent,
		name,
	} = at::get_file(newdirfd, linkpath.as_deref(), 0, true, true)?
	else {
		return Err(errno!(EEXIST));
	};
	let ts = current_time_sec(Clock::Realtime);
	vfs::symlink(
		&parent,
		name,
		target.as_bytes(),
		Stat {
			ctime: ts,
			mtime: ts,
			atime: ts,
			..Default::default()
		},
	)?;
	Ok(0)
}

pub fn readlink(pathname: UserString, buf: *mut u8, bufsiz: usize) -> EResult<usize> {
	// Get file
	let pathname = pathname.copy_path_from_user()?;
	let ent = vfs::get_file_from_path(&pathname, false)?;
	// Validation
	if ent.get_type()? != FileType::Link {
		return Err(errno!(EINVAL));
	}
	// Read link
	let buf = UserSlice::from_user(buf, bufsiz)?;
	let node = ent.node();
	let len = node.node_ops.readlink(node, buf)?;
	Ok(len as _)
}

pub fn open(pathname: UserString, flags: c_int, mode: file::Mode) -> EResult<usize> {
	do_openat(AT_FDCWD, pathname, flags, mode)
}

// TODO Implement all flags
// TODO rewrite doc
/// Returns the file at the given path.
///
/// Arguments:
/// - `dirfd` a file descriptor to the directory from which the file will be searched.
/// - `pathname` the path relative to the directory.
/// - `flags` is a set of open file flags.
/// - `mode` is the set of permissions to use if the file needs to be created.
///
/// If the file doesn't exist and the `O_CREAT` flag is set, the file is created,
/// then the function returns it.
///
/// If the flag is not set, the function returns an error with the appropriate errno.
///
/// If the file is to be created, the function uses `mode` to set its permissions.
fn get_file(
	dirfd: c_int,
	path: Option<&Path>,
	flags: c_int,
	mode: file::Mode,
) -> EResult<Arc<vfs::Entry>> {
	let creat = flags & O_CREAT != 0;
	let follow_link = flags & O_NOFOLLOW == 0;
	let resolved = at::get_file(dirfd, path, flags, creat, follow_link)?;
	match resolved {
		Resolved::Found(file) => Ok(file),
		Resolved::Creatable {
			parent,
			name,
		} => {
			let ts = current_time_sec(Clock::Realtime);
			vfs::create_file(
				parent,
				name,
				Stat {
					mode: FileType::Regular.to_mode() | mode,
					ctime: ts,
					mtime: ts,
					atime: ts,
					..Default::default()
				},
			)
		}
	}
}

/// Perform the `openat` system call.
pub fn do_openat(
	dirfd: c_int,
	pathname: UserString,
	flags: c_int,
	mode: file::Mode,
) -> EResult<usize> {
	let proc = Process::current();
	let pathname = pathname.copy_path_from_user()?;
	let mode = mode & !proc.umask();

	// Get file
	let file = get_file(dirfd, Some(&pathname), flags, mode)?;
	// Check permissions
	let (read, write) = match flags & 0b11 {
		O_RDONLY => (true, false),
		O_WRONLY => (false, true),
		O_RDWR => (true, true),
		_ => return Err(errno!(EINVAL)),
	};
	let ap = AccessProfile::cur_task();
	let stat = file.stat();
	if read && !ap.can_read_file(&stat) {
		return Err(errno!(EACCES));
	}
	if write && !ap.can_write_file(&stat) {
		return Err(errno!(EACCES));
	}
	let file_type = stat.get_type();
	// If `O_DIRECTORY` is set and the file is not a directory, return an error
	if flags & O_DIRECTORY != 0 && file_type != Some(FileType::Directory) {
		return Err(errno!(ENOTDIR));
	}
	// Open file
	const FLAGS_MASK: i32 =
		!(O_CLOEXEC | O_CREAT | O_DIRECTORY | O_EXCL | O_NOCTTY | O_NOFOLLOW | O_TRUNC);
	let file = File::open_entry(file, flags & FLAGS_MASK)?;
	// Truncate if necessary
	if flags & O_TRUNC != 0 && file_type == Some(FileType::Regular) {
		file.ops.truncate(&file, 0)?;
	}
	// Create FD
	let mut fd_flags = 0;
	if flags & O_CLOEXEC != 0 {
		fd_flags |= FD_CLOEXEC;
	}
	let (fd_id, _) = proc.file_descriptors().lock().create_fd(fd_flags, file)?;
	Ok(fd_id as _)
}

pub fn openat(
	dirfd: c_int,
	pathname: UserString,
	flags: c_int,
	mode: file::Mode,
) -> EResult<usize> {
	do_openat(dirfd, pathname, flags, mode)
}

/// Performs the access operation.
///
/// Arguments:
/// - `dirfd` is the file descriptor of the directory relative to which the check is done.
/// - `pathname` is the path to the file.
/// - `mode` is a bitfield of access permissions to check.
/// - `flags` is a set of flags.
/// - `rs` is the process's resolution settings.
/// - `fds_mutex` is the file descriptor table.
pub fn do_access(
	dirfd: Option<i32>,
	pathname: UserString,
	mode: i32,
	flags: i32,
) -> EResult<usize> {
	let pathname = pathname.copy_path_opt_from_user()?;
	let Resolved::Found(file) = at::get_file(
		dirfd.unwrap_or(AT_FDCWD),
		pathname.as_deref(),
		flags,
		false,
		true,
	)?
	else {
		unreachable!();
	};
	let ap = AccessProfile::cur_task();
	let stat = file.stat();
	let eaccess = flags & AT_EACCESS != 0;
	if (mode & R_OK != 0) && !ap.check_read_access(&stat, eaccess) {
		return Err(errno!(EACCES));
	}
	if (mode & W_OK != 0) && !ap.check_write_access(&stat, eaccess) {
		return Err(errno!(EACCES));
	}
	if (mode & X_OK != 0) && !ap.check_execute_access(&stat, eaccess) {
		return Err(errno!(EACCES));
	}
	Ok(0)
}

pub fn access(pathname: UserString, mode: c_int) -> EResult<usize> {
	do_access(None, pathname, mode, 0)
}

pub fn faccessat(dir_fd: c_int, pathname: UserString, mode: c_int) -> EResult<usize> {
	do_access(Some(dir_fd), pathname, mode, 0)
}

pub fn faccessat2(
	dir_fd: c_int,
	pathname: UserString,
	mode: c_int,
	flags: c_int,
) -> EResult<usize> {
	do_access(Some(dir_fd), pathname, mode, flags)
}

pub fn fadvise64_64(_fd: c_int, _offset: u64, _len: u64, _advice: c_int) -> EResult<usize> {
	// TODO
	Ok(0)
}

pub fn chmod(pathname: UserString, mode: file::Mode) -> EResult<usize> {
	fchmodat(AT_FDCWD, pathname, mode, 0)
}

pub fn fchmod(fd: c_int, mode: file::Mode) -> EResult<usize> {
	let file = fd_to_file(fd)?
		.vfs_entry
		.clone()
		.ok_or_else(|| errno!(EROFS))?;
	vfs::set_stat(
		file.node(),
		&StatSet {
			mode: Some(mode),
			..Default::default()
		},
	)?;
	Ok(0)
}

pub fn fchmodat(
	dirfd: c_int,
	pathname: UserString,
	mode: file::Mode,
	flags: c_int,
) -> EResult<usize> {
	let pathname = pathname.copy_path_opt_from_user()?;
	// Get file
	let Resolved::Found(file) = at::get_file(dirfd, pathname.as_deref(), flags, false, true)?
	else {
		unreachable!();
	};
	vfs::set_stat(
		file.node(),
		&StatSet {
			mode: Some(mode),
			..Default::default()
		},
	)?;
	Ok(0)
}

fn do_fchownat(
	dirfd: c_int,
	pathname: Option<UserString>,
	user: c_int,
	group: c_int,
	flags: c_int,
) -> EResult<usize> {
	// Validation
	if !(-1..=u16::MAX as c_int).contains(&user) || !(-1..=u16::MAX as c_int).contains(&group) {
		return Err(errno!(EINVAL));
	}
	let path = pathname
		.map(|pathname| pathname.copy_path_from_user())
		.transpose()?;
	// Get file
	let Resolved::Found(ent) = at::get_file(dirfd, path.as_deref(), flags, false, true)? else {
		unreachable!();
	};
	// TODO allow changing group to any group whose owner is member
	if !AccessProfile::cur_task().is_privileged() {
		return Err(errno!(EPERM));
	}
	vfs::set_stat(
		ent.node(),
		&StatSet {
			uid: (user > -1).then_some(user as _),
			gid: (group > -1).then_some(group as _),
			..Default::default()
		},
	)?;
	Ok(0)
}

pub fn chown(pathname: UserString, owner: c_int, group: c_int) -> EResult<usize> {
	do_fchownat(AT_FDCWD, Some(pathname), owner, group, 0)
}

pub fn lchown(pathname: UserString, owner: c_int, group: c_int) -> EResult<usize> {
	do_fchownat(AT_FDCWD, Some(pathname), owner, group, AT_SYMLINK_NOFOLLOW)
}

pub fn fchown(fd: c_int, owner: c_int, group: c_int) -> EResult<usize> {
	do_fchownat(fd, None, owner, group, AT_EMPTY_PATH)
}

pub fn fchownat(
	dirfd: c_int,
	path: UserString,
	owner: c_int,
	group: c_int,
	flags: c_int,
) -> EResult<usize> {
	do_fchownat(dirfd, Some(path), owner, group, flags)
}

pub fn getcwd(buf: *mut u8, size: usize) -> EResult<usize> {
	let buf = UserSlice::from_user(buf, size)?;
	let cwd = vfs::Entry::get_path(&Process::current().fs().lock().cwd)?;
	if unlikely(size < cwd.len() + 1) {
		return Err(errno!(ERANGE));
	}
	buf.copy_to_user(0, cwd.as_bytes())?;
	buf.copy_to_user(cwd.len(), b"\0")?;
	Ok(buf.as_ptr() as _)
}

pub fn chdir(path: UserString) -> EResult<usize> {
	let path = path.copy_path_from_user()?;
	// Get directory
	let dir = vfs::get_file_from_path(&path, true)?;
	// Validation
	let stat = dir.stat();
	if stat.get_type() != Some(FileType::Directory) {
		return Err(errno!(ENOTDIR));
	}
	if !AccessProfile::cur_task().can_list_directory(&stat) {
		return Err(errno!(EACCES));
	}
	// Set new cwd
	Process::current().fs().lock().cwd = dir;
	Ok(0)
}

pub fn chroot(path: UserString) -> EResult<usize> {
	let rs = ResolutionSettings {
		root: vfs::ROOT.clone(),
		..ResolutionSettings::cur_task(false, true)
	};
	// Check permission
	if !rs.access_profile.is_privileged() {
		return Err(errno!(EPERM));
	}
	let path = path.copy_path_from_user()?;
	// Get file
	let Resolved::Found(ent) = vfs::resolve_path(&path, &rs)? else {
		unreachable!();
	};
	if ent.get_type()? != FileType::Directory {
		return Err(errno!(ENOTDIR));
	}
	Process::current().fs().lock().chroot = ent;
	Ok(0)
}

pub fn fchdir(fd: c_int) -> EResult<usize> {
	let file = fd_to_file(fd)?
		.vfs_entry
		.clone()
		.ok_or_else(|| errno!(ENOTDIR))?;
	let stat = file.stat();
	// Check the file is an accessible directory
	if stat.get_type() != Some(FileType::Directory) {
		return Err(errno!(ENOTDIR));
	}
	if !AccessProfile::cur_task().can_list_directory(&stat) {
		return Err(errno!(EACCES));
	}
	Process::current().fs().lock().cwd = file;
	Ok(0)
}

pub fn umask(mask: file::Mode) -> EResult<usize> {
	let prev = Process::current().umask.swap(mask & 0o777, Release);
	Ok(prev as _)
}

pub fn utimensat(
	dirfd: c_int,
	pathname: UserString,
	times: UserPtr<[Timespec; 2]>,
	flags: c_int,
) -> EResult<usize> {
	let pathname = pathname.copy_path_opt_from_user()?;
	let (atime, mtime) = times
		.copy_from_user()?
		.map(|[atime, mtime]| (atime.to_nano(), mtime.to_nano()))
		.unwrap_or_else(|| {
			let ts = current_time_ns(Clock::Monotonic);
			(ts, ts)
		});
	// Get file
	let Resolved::Found(file) = at::get_file(dirfd, pathname.as_deref(), flags, false, true)?
	else {
		unreachable!();
	};
	// Update timestamps
	vfs::set_stat(
		file.node(),
		&StatSet {
			atime: Some(atime / 1_000_000_000),
			mtime: Some(mtime / 1_000_000_000),
			..Default::default()
		},
	)?;
	Ok(0)
}

pub fn rename(oldpath: UserString, newpath: UserString) -> EResult<usize> {
	do_renameat2(AT_FDCWD, oldpath, AT_FDCWD, newpath, 0)
}

// TODO implement flags
pub(super) fn do_renameat2(
	olddirfd: c_int,
	oldpath: UserString,
	newdirfd: c_int,
	newpath: UserString,
	_flags: c_int,
) -> EResult<usize> {
	// Get old file
	let oldpath = oldpath.copy_path_from_user()?;
	let Resolved::Found(old) = at::get_file(olddirfd, Some(&oldpath), 0, false, false)? else {
		unreachable!();
	};
	// Get new file
	let newpath = newpath.copy_path_from_user()?;
	let res = at::get_file(newdirfd, Some(&newpath), 0, true, true)?;
	match res {
		Resolved::Found(new) => {
			// cannot move the root of the vfs
			let new_parent = new.parent.clone().ok_or_else(|| errno!(EBUSY))?;
			vfs::rename(old, new_parent, &new.name)?;
		}
		Resolved::Creatable {
			parent: new_parent,
			name: new_name,
		} => vfs::rename(old, new_parent, new_name)?,
	}
	Ok(0)
}

pub fn renameat2(
	olddirfd: c_int,
	oldpath: UserString,
	newdirfd: c_int,
	newpath: UserString,
	flags: c_int,
) -> EResult<usize> {
	do_renameat2(olddirfd, oldpath, newdirfd, newpath, flags)
}

pub fn truncate(path: UserString, length: usize) -> EResult<usize> {
	let path = path.copy_path_from_user()?;
	let ent = vfs::get_file_from_path(&path, true)?;
	// Permission check
	if !AccessProfile::cur_task().can_write_file(&ent.stat()) {
		return Err(errno!(EACCES));
	}
	// Truncate
	let file = File::open_entry(ent, O_WRONLY)?;
	file.ops.truncate(&file, length as _)?;
	Ok(0)
}

pub fn unlink(pathname: UserString) -> EResult<usize> {
	do_unlinkat(AT_FDCWD, pathname, 0)
}

/// Perform the `unlinkat` system call.
pub fn do_unlinkat(dirfd: c_int, pathname: UserString, flags: c_int) -> EResult<usize> {
	let pathname = pathname.copy_path_from_user()?;
	// AT_EMPTY_PATH is required in case the path has only one component
	let resolved = at::get_file(dirfd, Some(&pathname), flags | AT_EMPTY_PATH, false, false)?;
	let Resolved::Found(ent) = resolved else {
		return Err(errno!(ENOENT));
	};
	vfs::unlink(ent)?;
	Ok(0)
}

pub fn unlinkat(dirfd: c_int, pathname: UserString, flags: c_int) -> EResult<usize> {
	do_unlinkat(dirfd, pathname, flags)
}

pub fn rmdir(pathname: UserString) -> EResult<usize> {
	let path = pathname.copy_path_from_user()?;
	let entry = vfs::get_file_from_path(&path, true)?;
	// Validation
	let stat = entry.get_type()?;
	if stat != FileType::Directory {
		return Err(errno!(ENOTDIR));
	}
	vfs::unlink(entry)?;
	Ok(0)
}
