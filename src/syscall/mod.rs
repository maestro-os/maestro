/// This module handles system calls. A system call is "function" that allows to communcate between
/// userspace and kernelspace.
/// TODO doc

mod _exit;
mod chroot;
mod close;
mod fork;
mod getpgid;
mod getpid;
mod getppid;
mod kill;
mod open;
mod read;
mod setpgid;
mod unlink;
mod waitpid;
mod write;

use crate::process::Process;
use crate::util::lock::mutex::MutexGuard;
use crate::process::signal;

use _exit::_exit;
use chroot::chroot;
use close::close;
use crate::util;
use fork::fork;
use getpgid::getpgid;
use getpid::getpid;
use getppid::getppid;
use kill::kill;
use open::open;
use read::read;
use setpgid::setpgid;
use unlink::unlink;
use waitpid::waitpid;
use write::write;

/// This function is called whenever a system call is triggered.
#[no_mangle]
pub extern "C" fn syscall_handler(regs: &util::Regs) -> u32 {
	let mut mutex = Process::get_current().unwrap();
	let mut guard = MutexGuard::new(&mut mutex);
	let curr_proc = guard.get_mut();
	curr_proc.set_regs(regs);
	// TODO Issue with functions that never return

	let id = regs.eax;

	match id {
		// TODO chown, chmod, mkdir, mknod, link, mount, ...
		0 => open(curr_proc, regs),
		1 => close(curr_proc, regs),
		2 => unlink(curr_proc, regs),
		3 => chroot(curr_proc, regs),
		4 => read(curr_proc, regs),
		5 => write(curr_proc, regs),
		6 => _exit(curr_proc, regs),
		7 => fork(curr_proc, regs),
		8 => waitpid(curr_proc, regs),
		9 => getpid(curr_proc, regs),
		10 => getppid(curr_proc, regs),
		11 => getpgid(curr_proc, regs),
		12 => setpgid(curr_proc, regs),
		13 => kill(curr_proc, regs),
		// TODO signal

		_ => {
			curr_proc.kill(signal::SIGSYS).unwrap(); // TODO Handle properly
			unsafe {
				crate::kernel_loop();
			}
		}
	}
}
