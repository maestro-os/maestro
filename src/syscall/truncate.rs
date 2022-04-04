//! The truncate syscall allows to truncate a file.

use crate::errno::Errno;
use crate::file::fcache;
use crate::file::path::Path;
use crate::process::Process;
use crate::process::regs::Regs;

/// The implementation of the `truncate` syscall.
pub fn truncate(regs: &Regs) -> Result<i32, Errno> {
	let path = regs.ebx as *const u8;
	let length = regs.ecx as usize;

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();
	let uid = proc.get_euid();
	let gid = proc.get_egid();

	let path = Path::from_str(super::util::get_str(proc, path)?, true)?;

	let mutex = fcache::get();
	let mut guard = mutex.lock();
	let files_cache = guard.get_mut();

	let file_mutex = files_cache.as_mut().unwrap().get_file_from_path(&path, uid, gid, true)?;
	let mut file_guard = file_mutex.lock();
	let file = file_guard.get_mut();
	file.set_size(length as _);

	Ok(0)
}
