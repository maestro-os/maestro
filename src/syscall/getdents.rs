//! The `getdents` system call allows to get the list of entries in a given directory.

use core::ffi::c_void;
use core::mem::size_of;
use core::ptr;
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
	let count = regs.edx as u32;

	if dirp.is_null() {
		return Err(errno!(EINVAL));
	}

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	// Checking access
	if !proc.get_mem_space().unwrap().can_access(dirp as _, count as _, true, false) {
		return Err(errno!(EFAULT));
	}

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
				// Skip entries if the inode cannot fit
				if entry.inode > u32::MAX as _ {
					continue;
				}

				let len = size_of::<LinuxDirent>() + entry.name.len() + 2;
				// If the buffer would overflow, return an error
				if off + len > count as _ {
					return Err(errno!(EINVAL));
				}

				let lin_ent = unsafe { // Safe because access has been checked before
					&mut *(dirp.add(off) as *mut LinuxDirent)
				};
				*lin_ent = LinuxDirent {
					d_ino: entry.inode as _,
					d_off: (off + len) as _,
					d_reclen: len as _,
					d_name: [],
				};

				unsafe {
					// Copying file name
					ptr::copy_nonoverlapping(entry.name.as_bytes().as_ptr(),
						lin_ent.d_name.as_mut_ptr(),
						entry.name.len());

					// Writing padding byte
					*lin_ent.d_name.as_mut_ptr().add(entry.name.len()) = 0;

					// Writing entry type
					let entry_type = entry.entry_type.to_dirent_type();
					*lin_ent.d_name.as_mut_ptr().add(entry.name.len() + 1) = entry_type;
				}

				off += len;
			}

			Ok(off as _)
		},

		_ => Err(errno!(ENOTDIR)),
	}
}
