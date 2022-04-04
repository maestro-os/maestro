//! The `getdents64` system call allows to get the list of entries in a given directory.

use core::ffi::c_void;
use core::mem::size_of;
use core::ptr;
use crate::errno::Errno;
use crate::file::FileContent;
use crate::file::file_descriptor::FDTarget;
use crate::process::Process;
use crate::process::regs::Regs;

/// Structure representing a Linux directory entry with 64 bits offsets.
#[repr(C)]
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
	let count = regs.edx as usize;

	if fd < 0 || dirp.is_null() {
		return Err(errno!(EBADF));
	}

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	// Checking access
	if !proc.get_mem_space().unwrap().can_access(dirp as _, count, true, true) {
		return Err(errno!(EFAULT));
	}

	// Getting file descriptor
	let fd = proc.get_fd(fd as _).ok_or(errno!(EBADF))?;

	let mut off = 0;
	let mut entries_count = 0;
	let start = fd.get_offset();

	{
		// Getting entries from the directory
		let fd_target = fd.get_target();
		let file_mutex = match fd_target {
			FDTarget::File(file) => file,
			_ => return Err(errno!(ENOTDIR)),
		};
		let file_guard = file_mutex.lock();
		let file = file_guard.get();
		let entries = match file.get_file_content() {
			FileContent::Directory(entries) => entries,
			_ => return Err(errno!(ENOTDIR)),
		};

		// Iterating over entries and filling the buffer
		for entry in &entries.as_slice()[(start as usize)..] {
			let len = size_of::<LinuxDirent64>() + entry.name.len() + 1;
			// If the buffer is not large enough, return an error
			if off == 0 && len > count {
				return Err(errno!(EINVAL));
			}
			// If reaching the end of the buffer, break
			if off + len > count {
				break;
			}

			let ent = unsafe { // Safe because access has been checked before
				&mut *(dirp.add(off) as *mut LinuxDirent64)
			};
			*ent = LinuxDirent64 {
				d_ino: entry.inode,
				d_off: (off + len) as _,
				d_reclen: len as _,
				d_type: entry.entry_type.to_dirent_type(),
				d_name: [],
			};

			// Copying file name
			unsafe {
				ptr::copy_nonoverlapping(entry.name.as_bytes().as_ptr(),
					ent.d_name.as_mut_ptr(),
					entry.name.len());

				// Writing padding byte
				*ent.d_name.as_mut_ptr().add(entry.name.len()) = 0;
			}

			off += len;
			entries_count += 1;
		}
	}

	fd.set_offset(start + entries_count);
	Ok(off as _)
}
