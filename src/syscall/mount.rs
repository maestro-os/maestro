//! The mount system call allows to mount a filesystem on the system.

use crate::errno;
use crate::errno::Errno;
use crate::file::fs;
use crate::file::mountpoint;
use crate::file::mountpoint::MountSource;
use crate::file::path::Path;
use crate::file::vfs;
use crate::file::FileType;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use crate::util::TryClone;
use core::ffi::c_ulong;
use core::ffi::c_void;
use macros::syscall;

#[syscall]
pub fn mount(
	source: SyscallString,
	target: SyscallString,
	filesystemtype: SyscallString,
	mountflags: c_ulong,
	_data: SyscallPtr<c_void>,
) -> Result<i32, Errno> {
	let (mount_source, fs_type, target_path) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let cwd = proc.chroot.try_clone()?.concat(proc.get_cwd())?;

		// Getting strings
		let source_slice = source.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;
		let target_slice = target.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;
		let filesystemtype_slice = filesystemtype
			.get(&mem_space_guard)?
			.ok_or(errno!(EFAULT))?;

		// Getting the mount source
		let mount_source = MountSource::from_str(source_slice, cwd)?;

		// Getting the target file
		let target_path = Path::from_str(target_slice, true)?;
		let target_path = super::util::get_absolute_path(&proc, target_path)?;
		let target_mutex = {
			let mut vfs = vfs::get().lock();
			let vfs = vfs.as_mut().unwrap();

			vfs.get_file_from_path(&target_path, proc.euid, proc.egid, true)?
		};
		let target_file = target_mutex.lock();

		// Checking the target is a directory
		if target_file.get_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}

		// TODO Check for loop between source and target

		let fs_type = fs::get_type(filesystemtype_slice).ok_or(errno!(ENODEV))?;

		(mount_source, fs_type, target_path)
	};

	// TODO Use `data`
	// Creating mountpoint
	mountpoint::create(mount_source, Some(fs_type), mountflags, target_path)?;

	Ok(0)
}
