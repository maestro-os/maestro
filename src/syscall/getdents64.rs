//! The `getdents64` system call allows to get the list of entries in a given
//! directory.

use crate::errno::Errno;
use crate::file::open_file::FDTarget;
use crate::file::FileContent;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::Process;
use core::ffi::c_int;
use core::ffi::c_void;
use core::mem::size_of;
use core::ptr;
use macros::syscall;

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
#[syscall]
pub fn getdents64(fd: c_int, dirp: SyscallSlice<c_void>, count: usize) -> Result<i32, Errno> {
	if fd < 0 {
		return Err(errno!(EBADF));
	}

	let (mem_space, open_file_mutex) = {
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let mem_space = proc.get_mem_space().unwrap();
		let open_file_mutex = proc
			.get_fd(fd as _)
			.ok_or_else(|| errno!(EBADF))?
			.get_open_file();

		(mem_space, open_file_mutex)
	};

	// Getting file
	let open_file_guard = open_file_mutex.lock();
	let open_file = open_file_guard.get_mut();

	let mem_space_guard = mem_space.lock();
	let dirp_slice = dirp
		.get_mut(&mem_space_guard, count)?
		.ok_or_else(|| errno!(EFAULT))?;

	let mut off = 0;
	let mut entries_count = 0;
	let start = open_file.get_offset();

	{
		// Getting entries from the directory
		let fd_target = open_file.get_target();
		let file_mutex = match fd_target {
			FDTarget::File(file) => file,
			_ => return Err(errno!(ENOTDIR)),
		};
		let file_guard = file_mutex.lock();
		let file = file_guard.get();
		let entries = match file.get_content() {
			FileContent::Directory(entries) => entries,
			_ => return Err(errno!(ENOTDIR)),
		};

		// Iterating over entries and filling the buffer
		for (name, entry) in entries.iter().skip(start as _) {
			let len = size_of::<LinuxDirent64>() + name.len() + 1;
			// If the buffer is not large enough, return an error
			if off == 0 && len > count {
				return Err(errno!(EINVAL));
			}
			// If reaching the end of the buffer, break
			if off + len > count {
				break;
			}

			let ent = unsafe {
				// Safe because access has been checked before
				&mut *(&mut dirp_slice[off] as *mut _ as *mut LinuxDirent64)
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
				ptr::copy_nonoverlapping(
					name.as_bytes().as_ptr(),
					ent.d_name.as_mut_ptr(),
					name.len(),
				);

				// Writing padding byte
				*ent.d_name.as_mut_ptr().add(name.len()) = 0;
			}

			off += len;
			entries_count += 1;
		}
	}

	open_file.set_offset(start + entries_count);
	Ok(off as _)
}
