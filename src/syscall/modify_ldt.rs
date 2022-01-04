//! This module implements the `set_thread_area` system call, which allows to set a LDT entry for
//! the process.

use core::ffi::c_void;
use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::Regs;
use crate::process::user_desc::UserDesc;
use crate::process::user_desc;

/// The implementation of the `set_thread_area` syscall.
pub fn modify_ldt(regs: &Regs) -> Result<i32, Errno> {
	let func = regs.ebx as i32;
	let ptr = regs.ecx as *mut c_void;
	let bytecount = regs.edx as u32;

	// Checking the given pointer is not null
	if ptr.is_null() {
		return Err(errno::EINVAL);
	}

	// Getting the current process
	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	// Checking the process can access the given pointer
	if !proc.get_mem_space().unwrap().can_access(ptr as _, bytecount as _, true, true) {
		return Err(errno::EFAULT);
	}

	match func {
		0 => {
			// TODO Read entry

			Ok(user_desc::USER_DESC_SIZE as _)
		},
		1 | 0x11 => {
			if bytecount != user_desc::USER_DESC_SIZE as _ {
				return Err(errno::EINVAL);
			}

			// A reference to the user_desc structure
			let info = unsafe { // Safe because the access was checked before
				UserDesc::from_ptr(ptr)
			};

			// TODO Add support for entry removal

			// The LDT descriptor
			let desc = info.to_descriptor();
			// The LDT
			let ldt = proc.get_ldt_mut()?;

			// Setting the entry and reloading the LDT
			ldt.set(info.get_entry_number() as _, desc)?;
			ldt.load();

			Ok(0)
		},
		2 => {
			// Zero-ing the pointer
			for i in 0..(bytecount as usize) {
				unsafe { // Safe because access to the pointer has been checked before
					*(ptr as *mut u8).add(i) = 0;
				}
			}

			Ok(bytecount as _)
		},

		_ => {
			return Err(errno::ENOSYS);
		},
	}
}
