/// This module handles system calls. A system call is "function" that allows to communcate between
/// userspace and kernelspace.
/// TODO doc

mod _exit;
mod getpid;
mod getppid;
mod write;

use _exit::_exit;
use crate::util;
use getpid::getpid;
use getppid::getppid;
use write::write;

/// This function is called whenever a system call is triggered.
#[no_mangle]
pub extern "C" fn syscall_handler(regs: &util::Regs) -> u32 {
	let id = regs.eax;
	match id {
		0 => write(regs),
		1 => _exit(regs),
		2 => getpid(regs),
		3 => getppid(regs),
		_ => {
			// TODO Kill process for invalid system call
			loop {}
		}
	}
}
