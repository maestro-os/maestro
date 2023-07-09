//! The `getdents` system call allows to get the list of entries in a given
//! directory.

use crate::errno::Errno;
use crate::file::FileContent;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::Process;
use core::ffi::c_uint;
use core::ffi::c_void;
use core::mem::offset_of;
use core::mem::size_of;
use core::ptr;
use macros::syscall;

/// Structure representing a Linux directory entry.
#[repr(C)]
struct LinuxDirent {
	/// Inode number.
	d_ino: u32,
	/// Offset to the next entry.
	d_off: u32,
	/// Length of this entry.
	d_reclen: u16,
	/// Filename (null-terminated).
	///
	/// The filename is immediately followed by a zero padding byte, then a byte
	/// indicating the type of the entry.
	d_name: [u8; 0],
}

#[syscall]
pub fn getdents(fd: c_uint, dirp: SyscallSlice<c_void>, count: c_uint) -> Result<i32, Errno> {
	let (mem_space, open_file_mutex) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();

		let fds_mutex = proc.get_fds().unwrap();
		let fds = fds_mutex.lock();

		let open_file_mutex = fds
			.get_fd(fd as _)
			.ok_or_else(|| errno!(EBADF))?
			.get_open_file()?;

		(mem_space, open_file_mutex)
	};

	// Getting file
	let mut open_file = open_file_mutex.lock();

	let mut mem_space_guard = mem_space.lock();
	let dirp_slice = dirp
		.get_mut(&mut mem_space_guard, count as _)?
		.ok_or_else(|| errno!(EFAULT))?;

	let mut off = 0;
	let mut entries_count = 0;
	let start = open_file.get_offset();

	{
		let file_mutex = open_file.get_file()?;
		let file = file_mutex.lock();

		let entries = match file.get_content() {
			FileContent::Directory(entries) => entries,
			_ => return Err(errno!(ENOTDIR)),
		};

		// Iterating over entries and filling the buffer
		for (name, entry) in entries.iter().skip(start as _) {
			// Skip entries if the inode cannot fit
			if entry.inode > u32::MAX as _ {
				continue;
			}

			let len = size_of::<LinuxDirent>() + name.len() + 2;
			// If the buffer is not large enough, return an error
			if off == 0 && len > count as usize {
				return Err(errno!(EINVAL));
			}
			// If reaching the end of the buffer, break
			if off + len > count as usize {
				break;
			}

			let ent_ptr = &mut dirp_slice[off] as *mut _ as *mut LinuxDirent;
			let ent = LinuxDirent {
				d_ino: entry.inode as _,
				d_off: (off + len) as _,
				d_reclen: len as _,
				d_name: [],
			};
			let ent_name_ptr =
				unsafe { (ent_ptr as *mut u8).add(offset_of!(LinuxDirent, d_name)) };

			unsafe {
				// Writing entry
				ptr::write_unaligned(ent_ptr, ent);

				// Copying file name
				ptr::copy_nonoverlapping(name.as_bytes().as_ptr(), ent_name_ptr, name.len());

				// Writing padding byte
				*ent_name_ptr.add(name.len()) = 0;

				// Writing entry type
				let entry_type = entry.entry_type.to_dirent_type();
				*ent_name_ptr.add(name.len() + 1) = entry_type;
			}

			off += len;
			entries_count += 1;
		}
	}

	open_file.set_offset(start + entries_count);
	Ok(off as _)
}
