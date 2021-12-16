//! The mount system call allows to mount a filesystem on the system.

use core::ffi::c_void;
use core::slice;
use crate::errno::Errno;
use crate::errno;
use crate::file::fs;
use crate::file::path::Path;
use crate::file;
use crate::process::Process;
use crate::process::Regs;

// TODO Move somewhere else to be used everywhere
/// Checks that the string at the given pointer `s` is accessible by the process `proc`, then
/// returns a slice to it.
/// If the string is not accessible by the process, the function returns an error.
fn get_str(proc: &Process, s: *const u8) -> Result<&'static [u8], Errno> {
	let mem_space = proc.get_mem_space().unwrap();

	// Getting the length and checking access
	let len = mem_space.can_access_string(s as _, true, false).ok_or(errno::EFAULT)?;
	// Getting the slice
	let slice = unsafe { // Safe because the access is checked before
		slice::from_raw_parts(s, len)
	};

	Ok(slice)
}

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
	let source_slice = get_str(proc, source)?;
	let target_slice = get_str(proc, target)?;
	let filesystemtype_slice = get_str(proc, filesystemtype)?;

	// TODO Handle non-file sources
	let _source_path = Path::from_str(source_slice, true)?;

	let target_path = Path::from_str(target_slice, true)?;
	let _target_file = {
		let cache = file::get_files_cache();
		let mut guard = cache.lock(false);

		guard.get_mut().as_mut().unwrap().get_file_from_path(&target_path)?
	};
	// TODO

	let _fs_type = fs::get_fs(filesystemtype_slice).ok_or(errno::ENODEV)?;

	// TODO

	Ok(0)
}
