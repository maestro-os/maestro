//! The `getdents64` system call allows to get the list of entries in a given directory.

use core::ffi::c_void;
use crate::errno::Errno;
use crate::file::FileContent;
use crate::file::file_descriptor::FDTarget;
use crate::process::Process;
use crate::process::Regs;

/// Structure representing a Linux directory entry with 64 bits offsets.
struct LinuxDirent64 {
	/// 64-bit inode number.
	d_ino: u64,
	/// 64-bit offset to next entry.
	d_off: u64,
	/// Size of this dirent.
	d_reclen: u16,
	/// File type.
	d_type: u8,
	/// Filename (null-terminated).
	d_name: [u8; 0],
}

/// The implementation of the `getdents64` syscall.
pub fn getdents64(regs: &Regs) -> Result<i32, Errno> {
	let fd = regs.ebx as i32;
	let dirp = regs.ecx as *mut c_void;
	let _count = regs.edx as usize;

	if fd < 0 || dirp.is_null() {
		return Err(errno!(EBADF));
	}

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	let fd = proc.get_fd(fd as _).ok_or(errno!(EBADF))?;
	let fd_target = fd.get_target();
	let file_mutex = match fd_target {
		FDTarget::File(file) => file,
		_ => return Err(errno!(ENOTDIR)),
	};

	let file_guard = file_mutex.lock();
	let file = file_guard.get();

	match file.get_file_content() {
		FileContent::Directory(entries) => {
			let mut off = 0;

			for entry in entries {
				let _lin_ent = LinuxDirent64 {
					d_ino: 0, // TODO
					d_off: 0, // TODO
					d_reclen: 0, // TODO
					d_type: 0, // TODO
					d_name: [], // TODO
				};

				// TODO
			}

			Ok(off)
		},

		_ => Err(errno!(ENOTDIR)),
	}
}
