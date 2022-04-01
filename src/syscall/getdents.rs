//! The `getdents` system call allows to get the list of entries in a given directory.

use core::ffi::c_void;
use crate::errno::Errno;
use crate::file::FileContent;
use crate::file::file_descriptor::FDTarget;
use crate::process::Process;
use crate::process::Regs;

/// Structure representing a Linux directory entry.
struct LinuxDirent {
	/// Inode number.
	d_ino: u32,
	/// Offset to the next entry.
	d_off: u32,
	/// Length of this entry.
	d_reclen: u16,
	/// Filename (null-terminated).
	/// The filename is immediately followed by a zero padding byte, then a byte indicating the
	/// type of the entry.
	d_name: [u8; 0],
}

/// The implementation of the `getdents` syscall.
pub fn getdents(regs: &Regs) -> Result<i32, Errno> {
	let fd = regs.ebx as u32;
	let dirp = regs.ecx as *mut c_void;
	let _count = regs.edx as u32;

	if dirp.is_null() {
		return Err(errno!(EINVAL));
	}

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	let fd = proc.get_fd(fd).ok_or(errno!(EBADF))?;
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
				let _lin_ent = LinuxDirent {
					d_ino: 0, // TODO
					d_off: 0, // TODO
					d_reclen: 0, // TODO
					d_name: [], // TODO
				};

				// TODO
			}

			Ok(off)
		},

		_ => return Err(errno!(ENOTDIR)),
	}
}
