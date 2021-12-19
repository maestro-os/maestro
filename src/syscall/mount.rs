//! The mount system call allows to mount a filesystem on the system.

use core::ffi::c_void;
use crate::errno::Errno;
use crate::errno;
use crate::file::FileType;
use crate::file::fs;
use crate::file::mountpoint::MountPoint;
use crate::file::mountpoint;
use crate::file::path::Path;
use crate::file;
use crate::process::Process;
use crate::process::Regs;

/// The implementation of the `mount` syscall.
pub fn mount(regs: &Regs) -> Result<i32, Errno> {
	let source = regs.ebx as *const u8;
	let target = regs.ecx as *const u8;
	let filesystemtype = regs.edx as *const u8;
	let mountflags = regs.esi as u32;
	let _data = regs.edi as *const c_void;

	// Getting slices to strings
	let (source_slice, target_slice, filesystemtype_slice) = {
		// Getting the process
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock(false);
		let proc = guard.get();

		// Getting strings
		let source_slice = super::util::get_str(proc, source)?;
		let target_slice = super::util::get_str(proc, target)?;
		let filesystemtype_slice = super::util::get_str(proc, filesystemtype)?;

		(source_slice, target_slice, filesystemtype_slice)
	};

	// TODO Handle non-file sources
	let source_path = Path::from_str(source_slice, true)?;
	let target_path = Path::from_str(target_slice, true)?;

	let (source_mutex, target_mutex) = {
		let mut guard = file::get_files_cache().lock(false);
		let fcache = guard.get_mut().as_mut().unwrap();

		let source = fcache.get_file_from_path(&source_path)?;
		let target = fcache.get_file_from_path(&target_path)?;

		(source, target)
	};

	// TODO Lock both at the same time. Implement an algorithm to avoid deadlocks
	let source_guard = source_mutex.lock(true);
	let target_guard = target_mutex.lock(true);

	let source_file = source_guard.get();
	let target_file = target_guard.get();

	// Checking the target is a directory
	if target_file.get_file_type() != FileType::Directory {
		return Err(errno::ENOTDIR);
	}

	// TODO Check for loop between source and target

	let fs_type = fs::get_fs(filesystemtype_slice).ok_or(errno::ENODEV)?;

	// TODO Use `data`
	// Creating mountpoint
	let mount = MountPoint::new(source_mutex, Some(fs_type), mountflags, target_path)?;
	mountpoint::register(mount)?;

	Ok(0)
}
