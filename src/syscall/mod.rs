//! This module handles system calls. A system call is "function" that allows to communcate between
//! userspace and kernelspace.
//! TODO doc

use crate::util::lock::mutex::TMutex;
use crate::process::Process;
use crate::process::signal;

mod _exit;
mod chroot;
mod close;
mod dup2;
mod dup;
mod fork;
mod getgid;
mod getpgid;
mod getpid;
mod getppid;
mod getuid;
mod kill;
mod open;
mod read;
mod setgid;
mod setpgid;
mod setuid;
mod umask;
mod uname;
mod unlink;
mod waitpid;
mod write;

use _exit::_exit;
use chroot::chroot;
use close::close;
use crate::util;
use dup2::dup2;
use dup::dup;
use fork::fork;
use getgid::getgid;
use getpgid::getpgid;
use getpid::getpid;
use getppid::getppid;
use getuid::getuid;
use kill::kill;
use open::open;
use read::read;
use setgid::setgid;
use setpgid::setpgid;
use setuid::setuid;
use umask::umask;
use uname::uname;
use unlink::unlink;
use waitpid::waitpid;
use write::write;

/// This function is called whenever a system call is triggered.
#[no_mangle]
pub extern "C" fn syscall_handler(regs: &util::Regs) -> u32 {
	let mut mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let curr_proc = guard.get_mut();
	curr_proc.set_regs(regs);
	// TODO Issue with functions that never return

	let id = regs.eax;

	let result = match id {
		0 => open(curr_proc, regs),
		1 => umask(curr_proc, regs),
		// TODO utime
		// TODO mkdir
		// TODO mknod
		// TODO pipe
		// TODO pipe2
		// TODO link
		// TODO fcntl
		2 => dup(curr_proc, regs),
		3 => dup2(curr_proc, regs),
		// TODO poll
		// TODO ppoll
		// TODO flock
		4 => close(curr_proc, regs),
		5 => unlink(curr_proc, regs),
		6 => chroot(curr_proc, regs),
		// TODO chdir
		// TODO chown
		// TODO chmod
		// TODO access
		// TODO stat
		// TODO fstat
		// TODO lstat
		// TODO lseek
		// TODO truncate
		// TODO ftruncate
		7 => read(curr_proc, regs),
		8 => write(curr_proc, regs),
		// TODO mount
		// TODO umount
		// TODO sync
		// TODO syncfs
		// TODO fsync
		// TODO fdatasync
		9 => _exit(curr_proc, regs),
		10 => fork(curr_proc, regs),
		11 => waitpid(curr_proc, regs),
		// TODO execl
		// TODO execlp
		// TODO execle
		// TODO execv
		// TODO execvp
		// TODO execvpe
		// TODO getpriority
		// TODO setpriority
		// TODO getrlimit
		// TODO setrlimit
		// TODO getrusage
		12 => getuid(curr_proc, regs),
		13 => setuid(curr_proc, regs),
		// TODO geteuid
		// TODO seteuid
		14 => getgid(curr_proc, regs),
		15 => setgid(curr_proc, regs),
		// TODO getegid
		// TODO setegid
		16 => getpid(curr_proc, regs),
		17 => getppid(curr_proc, regs),
		18 => getpgid(curr_proc, regs),
		19 => setpgid(curr_proc, regs),
		// TODO getsid
		// TODO setsid
		// TODO gettid
		// TODO mmap
		// TODO munmap
		// TODO mlock
		// TODO munlock
		// TODO mlockall
		// TODO munlockall
		// TODO mprotect
		// TODO signal
		20 => kill(curr_proc, regs),
		// TODO pause
		// TODO socket
		// TODO getsockname
		// TODO getsockopt
		// TODO setsockopt
		// TODO connect
		// TODO listen
		// TODO select
		// TODO send
		// TODO sendto
		// TODO sendmsg
		// TODO shutdown
		// TODO time
		// TODO times
		// TODO gettimeofday
		// TODO ptrace
		21 => uname(curr_proc, regs),
		// TODO reboot

		_ => {
			curr_proc.kill(signal::SIGSYS).unwrap(); // TODO Handle properly
			crate::enter_loop();
		}
	};

	if let Ok(val) = result {
		val as _
	} else {
		-result.unwrap_err() as _
	}
}
