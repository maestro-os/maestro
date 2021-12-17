//! The mount system call allows to mount a filesystem on the system.

use core::ffi::c_void;
use crate::errno::Errno;
use crate::errno;
use crate::file::FileType;
use crate::file::fs;
use crate::file::path::Path;
use crate::file;
use crate::process::Process;
use crate::process::Regs;

/// The implementation of the `mount` syscall.
pub fn mount(regs: &Regs) -> Result<i32, Errno> {
	let source = regs.ebx as *const u8;
	let target = regs.ecx as *const u8;
	let filesystemtype = regs.edx as *const u8;
	let _mountflags = regs.esi as u32;
	let _data = regs.edi as *const c_void;

	// Getting the process
	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock(false);
	let proc = guard.get();

	// Getting strings
	let source_slice = super::util::get_str(proc, source)?;
	let target_slice = super::util::get_str(proc, target)?;
	let filesystemtype_slice = super::util::get_str(proc, filesystemtype)?;

	// TODO Handle non-file sources
	let _source_path = Path::from_str(source_slice, true)?;

	let target_path = Path::from_str(target_slice, true)?;
	let target_file = {
		let cache = file::get_files_cache();
		let mut guard = cache.lock(false);

		guard.get_mut().as_mut().unwrap().get_file_from_path(&target_path)?
	};

	// TODO Avoid deadlock and race conditions
	/*let target_guard = target_file.lock(true);
	if target_guard.get().get_file_type() != FileType::Directory {
		return Err(errno::ENOTDIR);
	}*/

	// TODO Check for loop between source and target

	let _fs_type = fs::get_fs(filesystemtype_slice).ok_or(errno::ENODEV)?;

	// TODO

	Ok(0)
}
