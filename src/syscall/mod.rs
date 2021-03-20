/// This module handles system calls. A system call is "function" that allows to communcate between
/// userspace and kernelspace.
/// TODO doc

mod write;
mod _exit;

use _exit::_exit;
use crate::util;
use write::write;

/// This function is called whenever a system call is triggered.
#[no_mangle]
pub extern "C" fn syscall_handler(regs: &util::Regs) -> u32 {
	let id = regs.eax;
	match id {
		0 => write(regs),
		1 => _exit(regs),
		_ => {
			// TODO Kill process for invalid system call
			loop {}
		}
	}
}
