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
		fd::{FD_CLOEXEC, FileDescriptorTable},
		fs::StatSet,
		perm::AccessProfile,
		vfs,
		vfs::{ResolutionSettings, Resolved},
	},
	memory::user::{UserPtr, UserSlice, UserString},
	process::Process,
	sync::mutex::Mutex,
	syscall::{
		Args, Umask,
		util::{
			at,
			at::{AT_EACCESS, AT_EMPTY_PATH, AT_FDCWD},
		},
	},
	time::{
		clock::{Clock, current_time_ns, current_time_sec},
		unit::{TimeUnit, Timespec},
	},
};
use core::{ffi::c_int, hint::unlikely, ops::Deref, sync::atomic};
use utils::{
	collections::path::{Path, PathBuf},
	errno,
	errno::EResult,
	limits::SYMLINK_MAX,
	ptr::arc::Arc,
};

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

pub fn creat(Args((pathname, mode)): Args<(UserString, c_int)>) -> EResult<usize> {
	do_openat(AT_FDCWD, pathname, O_CREAT | O_WRONLY | O_TRUNC, mode as _)
}

pub fn mkdir(
	Args((pathname, mode)): Args<(UserString, file::Mode)>,
	rs: ResolutionSettings,
	umask: Umask,
) -> EResult<usize> {
	let path = pathname.copy_from_user()?.ok_or(errno!(EFAULT))?;
	let path = PathBuf::try_from(path)?;
	// If the path is not empty, create
	if let Some(name) = path.file_name() {
		// Get parent directory
		let parent_path = path.parent().unwrap_or(Path::root());
		let parent = vfs::get_file_from_path(parent_path, &rs)?;
		let mode = mode & !umask.0;
		let ts = current_time_sec(Clock::Realtime);
		// Create the directory
		vfs::create_file(
			parent,
			name,
			&rs.access_profile,
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

pub fn mknod(
	Args((pathname, mode, dev)): Args<(UserString, file::Mode, u64)>,
	umask: Umask,
	rs: ResolutionSettings,
) -> EResult<usize> {
	let path = pathname.copy_from_user()?.ok_or(errno!(EFAULT))?;
	let path = PathBuf::try_from(path)?;
	let parent_path = path.parent().unwrap_or(Path::root());
	// File name
	let Some(name) = path.file_name() else {
		return Err(errno!(EEXIST));
	};
	// Check file type and permissions
	let mode = mode & !umask.0;
	let file_type = FileType::from_mode(mode).ok_or(errno!(EPERM))?;
	let privileged = rs.access_profile.is_privileged();
	match (file_type, privileged) {
		(FileType::Regular | FileType::Fifo | FileType::Socket, _) => {}
		(FileType::BlockDevice | FileType::CharDevice, true) => {}
		(_, false) => return Err(errno!(EPERM)),
		(_, true) => return Err(errno!(EINVAL)),
	}
	// Create file
	let ts = current_time_sec(Clock::Realtime);
	let parent = vfs::get_file_from_path(parent_path, &rs)?;
	vfs::create_file(
		parent,
		name,
		&rs.access_profile,
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

pub fn link(
	Args((oldpath, newpath)): Args<(UserString, UserString)>,
	fds_mutex: Arc<Mutex<FileDescriptorTable>>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	linkat(
		Args((AT_FDCWD, oldpath, AT_FDCWD, newpath, 0)),
		fds_mutex,
		rs,
	)
}

pub fn linkat(
	Args((olddirfd, oldpath, newdirfd, newpath, flags)): Args<(
		c_int,
		UserString,
		c_int,
		UserString,
		c_int,
	)>,
	fds_mutex: Arc<Mutex<FileDescriptorTable>>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	let oldpath = oldpath
		.copy_from_user()?
		.map(PathBuf::try_from)
		.ok_or_else(|| errno!(EFAULT))??;
	let newpath = newpath
		.copy_from_user()?
		.map(PathBuf::try_from)
		.ok_or_else(|| errno!(EFAULT))??;
	let fds = fds_mutex.lock();
	let rs = ResolutionSettings {
		follow_link: false,
		..rs
	};
	// Get old file
	let Resolved::Found(old) = at::get_file(&fds, rs.clone(), olddirfd, Some(&oldpath), flags)?
	else {
		return Err(errno!(ENOENT));
	};
	if old.get_type()? == FileType::Directory {
		return Err(errno!(EPERM));
	}
	// Create new file
	let rs = ResolutionSettings {
		create: true,
		..rs
	};
	let Resolved::Creatable {
		parent: new_parent,
		name: new_name,
	} = at::get_file(&fds, rs.clone(), newdirfd, Some(&newpath), 0)?
	else {
		return Err(errno!(EEXIST));
	};
	let name = new_name.try_into()?;
	vfs::link(&new_parent, name, old.node().clone(), &rs.access_profile)?;
	Ok(0)
}

pub fn symlink(
	Args((target, linkpath)): Args<(UserString, UserString)>,
	rs: ResolutionSettings,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	symlinkat(Args((target, AT_FDCWD, linkpath)), rs, fds)
}

pub fn symlinkat(
	Args((target, newdirfd, linkpath)): Args<(UserString, c_int, UserString)>,
	rs: ResolutionSettings,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	let target_slice = target.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
	if target_slice.len() > SYMLINK_MAX {
		return Err(errno!(ENAMETOOLONG));
	}
	let target = PathBuf::try_from(target_slice)?;
	let linkpath = linkpath
		.copy_from_user()?
		.map(PathBuf::try_from)
		.transpose()?;
	let rs = ResolutionSettings {
		create: true,
		follow_link: true,
		..rs
	};
	// Create link
	let Resolved::Creatable {
		parent,
		name,
	} = at::get_file(&fds.lock(), rs.clone(), newdirfd, linkpath.as_deref(), 0)?
	else {
		return Err(errno!(EEXIST));
	};
	let ts = current_time_sec(Clock::Realtime);
	vfs::symlink(
		&parent,
		name,
		target.as_bytes(),
		&rs.access_profile,
		Stat {
			ctime: ts,
			mtime: ts,
			atime: ts,
			..Default::default()
		},
	)?;
	Ok(0)
}

pub fn readlink(
	Args((pathname, buf, bufsiz)): Args<(UserString, *mut u8, usize)>,
) -> EResult<usize> {
	let proc = Process::current();
	// Get file
	let path = pathname.copy_from_user()?.ok_or(errno!(EFAULT))?;
	let path = PathBuf::try_from(path)?;
	let rs = ResolutionSettings::for_process(&proc, false);
	let ent = vfs::get_file_from_path(&path, &rs)?;
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

pub fn open(
	Args((pathname, flags, mode)): Args<(UserString, c_int, file::Mode)>,
) -> EResult<usize> {
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
	fds: &FileDescriptorTable,
	dirfd: c_int,
	path: Option<&Path>,
	flags: c_int,
	rs: ResolutionSettings,
	mode: file::Mode,
) -> EResult<Arc<vfs::Entry>> {
	let resolved = at::get_file(fds, rs.clone(), dirfd, path, flags)?;
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
				&rs.access_profile,
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
	let (rs, pathname, fds_mutex, mode) = {
		let proc = Process::current();
		let follow_link = flags & O_NOFOLLOW == 0;
		let rs = ResolutionSettings {
			create: flags & O_CREAT != 0,
			..ResolutionSettings::for_process(&proc, follow_link)
		};
		let pathname = pathname
			.copy_from_user()?
			.map(PathBuf::try_from)
			.ok_or_else(|| errno!(EFAULT))??;
		let fds_mutex = proc.file_descriptors.deref().clone().unwrap();
		let mode = mode & !proc.fs().lock().umask();
		(rs, pathname, fds_mutex, mode)
	};

	let mut fds = fds_mutex.lock();

	// Get file
	let file = get_file(&fds, dirfd, Some(&pathname), flags, rs.clone(), mode)?;
	// Check permissions
	let (read, write) = match flags & 0b11 {
		O_RDONLY => (true, false),
		O_WRONLY => (false, true),
		O_RDWR => (true, true),
		_ => return Err(errno!(EINVAL)),
	};
	let stat = file.stat();
	if read && !rs.access_profile.can_read_file(&stat) {
		return Err(errno!(EACCES));
	}
	if write && !rs.access_profile.can_write_file(&stat) {
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
	let (fd_id, _) = fds.create_fd(fd_flags, file)?;
	Ok(fd_id as _)
}

pub fn openat(
	Args((dirfd, pathname, flags, mode)): Args<(c_int, UserString, c_int, file::Mode)>,
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
	flags: Option<i32>,
	rs: ResolutionSettings,
	fds_mutex: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	let flags = flags.unwrap_or(0);
	// Use effective IDs instead of real IDs
	let eaccess = flags & AT_EACCESS != 0;
	let ap = rs.access_profile;
	let file = {
		let fds = fds_mutex.lock();
		let pathname = pathname
			.copy_from_user()?
			.map(PathBuf::try_from)
			.transpose()?;
		let Resolved::Found(file) = at::get_file(
			&fds,
			rs,
			dirfd.unwrap_or(AT_FDCWD),
			pathname.as_deref(),
			flags,
		)?
		else {
			return Err(errno!(ENOENT));
		};
		file
	};
	// Do access checks
	let stat = file.stat();
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

pub fn access(
	Args((pathname, mode)): Args<(UserString, c_int)>,
	rs: ResolutionSettings,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	do_access(None, pathname, mode, None, rs, fds)
}

pub fn faccessat(
	Args((dir_fd, pathname, mode)): Args<(c_int, UserString, c_int)>,
	rs: ResolutionSettings,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	do_access(Some(dir_fd), pathname, mode, None, rs, fds)
}

pub fn faccessat2(
	Args((dir_fd, pathname, mode, flags)): Args<(c_int, UserString, c_int, c_int)>,
	rs: ResolutionSettings,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	do_access(Some(dir_fd), pathname, mode, Some(flags), rs, fds)
}

pub fn fadvise64_64(
	Args((_fd, _offset, _len, _advice)): Args<(c_int, u64, u64, c_int)>,
) -> EResult<usize> {
	// TODO
	Ok(0)
}

pub fn chmod(
	Args((pathname, mode)): Args<(UserString, file::Mode)>,
	fds_mutex: Arc<Mutex<FileDescriptorTable>>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	fchmodat(Args((AT_FDCWD, pathname, mode, 0)), fds_mutex, rs)
}

pub fn fchmod(
	Args((fd, mode)): Args<(c_int, file::Mode)>,
	fds_mutex: Arc<Mutex<FileDescriptorTable>>,
	ap: AccessProfile,
) -> EResult<usize> {
	let file = fds_mutex
		.lock()
		.get_fd(fd)?
		.get_file()
		.vfs_entry
		.clone()
		.ok_or_else(|| errno!(EROFS))?;
	// Check permissions
	let stat = file.stat();
	if !ap.can_set_file_permissions(&stat) {
		return Err(errno!(EPERM));
	}
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
	Args((dirfd, pathname, mode, flags)): Args<(c_int, UserString, file::Mode, c_int)>,
	fds_mutex: Arc<Mutex<FileDescriptorTable>>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	let pathname = pathname
		.copy_from_user()?
		.map(PathBuf::try_from)
		.transpose()?;
	// Get file
	let fds = fds_mutex.lock();
	let Resolved::Found(file) = at::get_file(&fds, rs.clone(), dirfd, pathname.as_deref(), flags)?
	else {
		return Err(errno!(ENOENT));
	};
	// Check permission
	let stat = file.stat();
	if !rs.access_profile.can_set_file_permissions(&stat) {
		return Err(errno!(EPERM));
	}
	vfs::set_stat(
		file.node(),
		&StatSet {
			mode: Some(mode),
			..Default::default()
		},
	)?;
	Ok(0)
}

/// Performs the `chown` syscall.
pub fn do_chown(
	pathname: UserString,
	owner: c_int,
	group: c_int,
	rs: ResolutionSettings,
) -> EResult<usize> {
	// Validation
	if !(-1..=u16::MAX as c_int).contains(&owner) || !(-1..=u16::MAX as c_int).contains(&group) {
		return Err(errno!(EINVAL));
	}
	let path = pathname.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
	let path = PathBuf::try_from(path)?;
	// Get file
	let ent = vfs::get_file_from_path(&path, &rs)?;
	// TODO allow changing group to any group whose owner is member
	if !rs.access_profile.is_privileged() {
		return Err(errno!(EPERM));
	}
	vfs::set_stat(
		ent.node(),
		&StatSet {
			uid: (owner > -1).then_some(owner as _),
			gid: (group > -1).then_some(group as _),
			..Default::default()
		},
	)?;
	Ok(0)
}

pub fn chown(
	Args((pathname, owner, group)): Args<(UserString, c_int, c_int)>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	do_chown(pathname, owner, group, rs)
}

pub fn lchown(
	Args((pathname, owner, group)): Args<(UserString, c_int, c_int)>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	do_chown(
		pathname,
		owner,
		group,
		ResolutionSettings {
			follow_link: false,
			..rs
		},
	)
}

pub fn getcwd(Args((buf, size)): Args<(*mut u8, usize)>, proc: Arc<Process>) -> EResult<usize> {
	let buf = UserSlice::from_user(buf, size)?;
	let cwd = vfs::Entry::get_path(&proc.fs().lock().cwd)?;
	if unlikely(size < cwd.len() + 1) {
		return Err(errno!(ERANGE));
	}
	buf.copy_to_user(0, cwd.as_bytes())?;
	buf.copy_to_user(cwd.len(), b"\0")?;
	Ok(buf.as_ptr() as _)
}

pub fn chdir(
	Args(path): Args<UserString>,
	proc: Arc<Process>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	let path = path.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
	let path = PathBuf::try_from(path)?;
	// Get directory
	let dir = vfs::get_file_from_path(&path, &rs)?;
	// Validation
	let stat = dir.stat();
	if stat.get_type() != Some(FileType::Directory) {
		return Err(errno!(ENOTDIR));
	}
	if !rs.access_profile.can_list_directory(&stat) {
		return Err(errno!(EACCES));
	}
	// Set new cwd
	proc.fs().lock().cwd = dir;
	Ok(0)
}

pub fn chroot(
	Args(path): Args<UserString>,
	proc: Arc<Process>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	// Check permission
	if !rs.access_profile.is_privileged() {
		return Err(errno!(EPERM));
	}
	let path = path.copy_from_user()?.ok_or(errno!(EFAULT))?;
	let path = PathBuf::try_from(path)?;
	let rs = ResolutionSettings {
		root: vfs::ROOT.clone(),
		..rs
	};
	// Get file
	let ent = vfs::get_file_from_path(&path, &rs)?;
	if ent.get_type()? != FileType::Directory {
		return Err(errno!(ENOTDIR));
	}
	proc.fs().lock().chroot = ent;
	Ok(0)
}

pub fn fchdir(
	Args(fd): Args<c_int>,
	fds: Arc<Mutex<FileDescriptorTable>>,
	ap: AccessProfile,
	proc: Arc<Process>,
) -> EResult<usize> {
	let file = fds
		.lock()
		.get_fd(fd)?
		.get_file()
		.vfs_entry
		.clone()
		.ok_or_else(|| errno!(ENOTDIR))?;
	let stat = file.stat();
	// Check the file is an accessible directory
	if stat.get_type() != Some(FileType::Directory) {
		return Err(errno!(ENOTDIR));
	}
	if !ap.can_list_directory(&stat) {
		return Err(errno!(EACCES));
	}
	proc.fs().lock().cwd = file;
	Ok(0)
}

pub fn umask(Args(mask): Args<file::Mode>, proc: Arc<Process>) -> EResult<usize> {
	let prev = proc
		.fs()
		.lock()
		.umask
		.swap(mask & 0o777, atomic::Ordering::Relaxed);
	Ok(prev as _)
}

pub fn utimensat(
	Args((dirfd, pathname, times, flags)): Args<(
		c_int,
		UserString,
		UserPtr<[Timespec; 2]>,
		c_int,
	)>,
	rs: ResolutionSettings,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	let pathname = pathname
		.copy_from_user()?
		.map(PathBuf::try_from)
		.transpose()?;
	let (atime, mtime) = times
		.copy_from_user()?
		.map(|[atime, mtime]| (atime.to_nano(), mtime.to_nano()))
		.unwrap_or_else(|| {
			let ts = current_time_ns(Clock::Monotonic);
			(ts, ts)
		});
	// Get file
	let Resolved::Found(file) = at::get_file(&fds.lock(), rs, dirfd, pathname.as_deref(), flags)?
	else {
		return Err(errno!(ENOENT));
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

pub fn rename(
	Args((oldpath, newpath)): Args<(UserString, UserString)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	do_renameat2(AT_FDCWD, oldpath, AT_FDCWD, newpath, 0, fds, rs)
}

// TODO implement flags
pub(super) fn do_renameat2(
	olddirfd: c_int,
	oldpath: UserString,
	newdirfd: c_int,
	newpath: UserString,
	_flags: c_int,
	fds: Arc<Mutex<FileDescriptorTable>>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	let rs = ResolutionSettings {
		follow_link: false,
		..rs
	};
	// Get old file
	let oldpath = oldpath
		.copy_from_user()?
		.map(PathBuf::try_from)
		.ok_or_else(|| errno!(EFAULT))??;
	let Resolved::Found(old) = at::get_file(&fds.lock(), rs.clone(), olddirfd, Some(&oldpath), 0)?
	else {
		return Err(errno!(ENOENT));
	};
	// Get new file
	let newpath = newpath
		.copy_from_user()?
		.map(PathBuf::try_from)
		.ok_or_else(|| errno!(EFAULT))??;
	let rs = ResolutionSettings {
		create: true,
		..rs
	};
	let res = at::get_file(&fds.lock(), rs.clone(), newdirfd, Some(&newpath), 0)?;
	match res {
		Resolved::Found(new) => {
			// cannot move the root of the vfs
			let new_parent = new.parent.clone().ok_or_else(|| errno!(EBUSY))?;
			vfs::rename(old, new_parent, &new.name, &rs.access_profile)?;
		}
		Resolved::Creatable {
			parent: new_parent,
			name: new_name,
		} => vfs::rename(old, new_parent, new_name, &rs.access_profile)?,
	}
	Ok(0)
}

pub fn renameat2(
	Args((olddirfd, oldpath, newdirfd, newpath, flags)): Args<(
		c_int,
		UserString,
		c_int,
		UserString,
		c_int,
	)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	do_renameat2(olddirfd, oldpath, newdirfd, newpath, flags, fds, rs)
}

pub fn truncate(Args((path, length)): Args<(UserString, usize)>) -> EResult<usize> {
	let proc = Process::current();
	let rs = ResolutionSettings::for_process(&proc, true);
	let path = path.copy_from_user()?.ok_or(errno!(EFAULT))?;
	let path = PathBuf::try_from(path)?;
	let ent = vfs::get_file_from_path(&path, &rs)?;
	// Permission check
	if !rs.access_profile.can_write_file(&ent.stat()) {
		return Err(errno!(EACCES));
	}
	// Truncate
	let file = File::open_entry(ent, O_WRONLY)?;
	file.ops.truncate(&file, length as _)?;
	Ok(0)
}

pub fn unlink(
	Args(pathname): Args<UserString>,
	rs: ResolutionSettings,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	do_unlinkat(AT_FDCWD, pathname, 0, rs, fds)
}

/// Perform the `unlinkat` system call.
pub fn do_unlinkat(
	dirfd: c_int,
	pathname: UserString,
	flags: c_int,
	rs: ResolutionSettings,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	let pathname = pathname
		.copy_from_user()?
		.map(PathBuf::try_from)
		.ok_or_else(|| errno!(EFAULT))??;
	let rs = ResolutionSettings {
		follow_link: false,
		..rs
	};
	// AT_EMPTY_PATH is required in case the path has only one component
	let resolved = at::get_file(
		&fds.lock(),
		rs.clone(),
		dirfd,
		Some(&pathname),
		flags | AT_EMPTY_PATH,
	)?;
	let Resolved::Found(ent) = resolved else {
		return Err(errno!(ENOENT));
	};
	vfs::unlink(ent, &rs.access_profile)?;
	Ok(0)
}

pub fn unlinkat(
	Args((dirfd, pathname, flags)): Args<(c_int, UserString, c_int)>,
	rs: ResolutionSettings,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	do_unlinkat(dirfd, pathname, flags, rs, fds)
}

pub fn rmdir(Args(pathname): Args<UserString>, rs: ResolutionSettings) -> EResult<usize> {
	let path = pathname.copy_from_user()?.ok_or(errno!(EFAULT))?;
	let path = PathBuf::try_from(path)?;
	let entry = vfs::get_file_from_path(&path, &rs)?;
	// Validation
	let stat = entry.get_type()?;
	if stat != FileType::Directory {
		return Err(errno!(ENOTDIR));
	}
	vfs::unlink(entry, &rs.access_profile)?;
	Ok(0)
}
