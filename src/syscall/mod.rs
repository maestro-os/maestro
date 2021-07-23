//! This module handles system calls. A system call is "function" that allows to communcate between
//! userspace and kernelspace.
//! TODO doc

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
mod reboot;
mod setgid;
mod setpgid;
mod setuid;
mod signal;
mod umask;
mod uname;
mod unlink;
mod waitpid;
mod write;

use crate::process::Process;
use crate::process;

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
use reboot::reboot;
use setgid::setgid;
use setpgid::setpgid;
use setuid::setuid;
use signal::signal;
use umask::umask;
use uname::uname;
use unlink::unlink;
use waitpid::waitpid;
use write::write;

/// This function is called whenever a system call is triggered.
#[no_mangle]
pub extern "C" fn syscall_handler(regs: &util::Regs) -> u32 {
	let id = regs.eax;

	let result = match id {
		0 => open(regs),
		1 => umask(regs),
		// TODO utime
		// TODO mkdir
		// TODO mknod
		// TODO pipe
		// TODO pipe2
		// TODO link
		// TODO fcntl
		2 => dup(regs),
		3 => dup2(regs),
		// TODO poll
		// TODO ppoll
		// TODO flock
		4 => close(regs),
		5 => unlink(regs),
		6 => chroot(regs),
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
		7 => read(regs),
		8 => write(regs),
		// TODO mount
		// TODO umount
		// TODO sync
		// TODO syncfs
		// TODO fsync
		// TODO fdatasync
		9 => _exit(regs),
		10 => fork(regs),
		11 => waitpid(regs),
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
		12 => getuid(regs),
		13 => setuid(regs),
		// TODO geteuid
		// TODO seteuid
		14 => getgid(regs),
		15 => setgid(regs),
		// TODO getegid
		// TODO setegid
		16 => getpid(regs),
		17 => getppid(regs),
		18 => getpgid(regs),
		19 => setpgid(regs),
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
		20 => signal(regs),
		21 => kill(regs),
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
		22 => uname(regs),
		23 => reboot(regs),

		_ => {
			let mut mutex = Process::get_current().unwrap();
			let mut guard = mutex.lock(false); // TODO Make locking inside of the syscall handler itself
			let curr_proc = guard.get_mut();

			curr_proc.kill(process::signal::Signal::new(process::signal::SIGSYS).unwrap());
			crate::enter_loop();
		}
	};

	if let Ok(val) = result {
		val as _
	} else {
		-result.unwrap_err() as _
	}
}
