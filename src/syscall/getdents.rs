//! The `getdents` system call allows to get the list of entries in a given directory.

use crate::errno::Errno;
use crate::file::open_file::FDTarget;
use crate::file::FileContent;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::regs::Regs;
use crate::process::Process;
use core::ffi::c_void;
use core::mem::size_of;
use core::ptr;

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
	/// The filename is immediately followed by a zero padding byte, then a byte indicating the
	/// type of the entry.
	d_name: [u8; 0],
}

/// The implementation of the `getdents` syscall.
pub fn getdents(regs: &Regs) -> Result<i32, Errno> {
	let fd = regs.ebx as u32;
	let dirp: SyscallSlice<c_void> = (regs.ecx as usize).into();
	let count = regs.edx as u32;

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
		.get_mut(&mem_space_guard, count as _)?
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

			let ent = unsafe {
				// Safe because access has been checked before
				&mut *(&mut dirp_slice[off] as *mut _ as *mut LinuxDirent)
			};
			*ent = LinuxDirent {
				d_ino: entry.inode as _,
				d_off: (off + len) as _,
				d_reclen: len as _,
				d_name: [],
			};

			unsafe {
				// Copying file name
				ptr::copy_nonoverlapping(
					name.as_bytes().as_ptr(),
					ent.d_name.as_mut_ptr(),
					name.len(),
				);

				// Writing padding byte
				*ent.d_name.as_mut_ptr().add(name.len()) = 0;

				// Writing entry type
				let entry_type = entry.entry_type.to_dirent_type();
				*ent.d_name.as_mut_ptr().add(name.len() + 1) = entry_type;
			}

			off += len;
			entries_count += 1;
		}
	}

	open_file.set_offset(start + entries_count);
	Ok(off as _)
}
