/// This module handles system calls. A system call is "function" that allows to communcate between
/// userspace and kernelspace.
/// TODO doc

mod _exit;
mod chroot;
mod close;
mod fork;
mod getpid;
mod getppid;
mod open;
mod read;
mod unlink;
mod waitpid;
mod write;

use _exit::_exit;
use chroot::chroot;
use close::close;
use crate::util;
use fork::fork;
use getpid::getpid;
use getppid::getppid;
use open::open;
use read::read;
use unlink::unlink;
use waitpid::waitpid;
use write::write;

/// This function is called whenever a system call is triggered.
#[no_mangle]
pub extern "C" fn syscall_handler(regs: &util::Regs) -> u32 {
	let id = regs.eax;
	match id {
		// TODO chown, chmod, mkdir, mknod, link, mount, ...
		0 => open(regs),
		1 => close(regs),
		2 => unlink(regs),
		3 => chroot(regs),
		4 => read(regs),
		5 => write(regs),
		6 => _exit(regs),
		7 => fork(regs),
		8 => waitpid(regs),
		9 => getpid(regs),
		10 => getppid(regs),
		_ => {
			// TODO Kill process for invalid system call
			loop {}
		}
	}
}
