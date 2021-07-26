//! This module handles system calls. A system call is "function" that allows to communcate between
//! userspace and kernelspace.
//! TODO doc

mod _exit;
mod brk;
mod chdir;
mod chroot;
mod close;
mod dup2;
mod dup;
mod fchdir;
mod fork;
mod getcwd;
mod getgid;
mod getpgid;
mod getpid;
mod getppid;
mod getuid;
mod kill;
mod mmap;
mod munmap;
mod open;
//mod pipe2;
mod pipe;
mod read;
mod reboot;
mod sbrk;
mod setgid;
mod setpgid;
mod setuid;
mod signal;
mod umask;
mod uname;
mod unlink;
mod wait;
mod waitpid;
mod write;

use crate::process::Process;
use crate::process;

use _exit::_exit;
use brk::brk;
use chdir::chdir;
use chroot::chroot;
use close::close;
use crate::util;
use dup2::dup2;
use dup::dup;
use fchdir::fchdir;
use fork::fork;
use getcwd::getcwd;
use getgid::getgid;
use getpgid::getpgid;
use getpid::getpid;
use getppid::getppid;
use getuid::getuid;
use kill::kill;
use mmap::mmap;
use munmap::munmap;
use open::open;
//use pipe2::pipe2;
use pipe::pipe;
use read::read;
use reboot::reboot;
use sbrk::sbrk;
use setgid::setgid;
use setpgid::setpgid;
use setuid::setuid;
use signal::signal;
use umask::umask;
use uname::uname;
use unlink::unlink;
use wait::wait;
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
		2 => pipe(regs),
		//3 => pipe2(regs),
		// TODO link
		// TODO fcntl
		4 => dup(regs),
		5 => dup2(regs),
		// TODO poll
		// TODO ppoll
		// TODO flock
		6 => close(regs),
		7 => unlink(regs),
		8 => chroot(regs),
		9 => getcwd(regs),
		10 => chdir(regs),
		11 => fchdir(regs),
		// TODO chown
		// TODO fchown
		// TODO lchown
		// TODO chmod
		// TODO fchmod
		// TODO access
		// TODO stat
		// TODO fstat
		// TODO lstat
		// TODO lseek
		// TODO truncate
		// TODO ftruncate
		12 => read(regs),
		13 => write(regs),
		// TODO mount
		// TODO umount
		// TODO sync
		// TODO syncfs
		// TODO fsync
		// TODO fdatasync
		14 => _exit(regs),
		15 => fork(regs),
		16 => wait(regs),
		17 => waitpid(regs),
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
		18 => getuid(regs),
		19 => setuid(regs),
		// TODO geteuid
		// TODO seteuid
		20 => getgid(regs),
		21 => setgid(regs),
		// TODO getegid
		// TODO setegid
		22 => getpid(regs),
		23 => getppid(regs),
		24 => getpgid(regs),
		25 => setpgid(regs),
		// TODO getsid
		// TODO setsid
		// TODO gettid
		26 => brk(regs),
		27 => sbrk(regs),
		28 => mmap(regs),
		29 => munmap(regs),
		// TODO mlock
		// TODO munlock
		// TODO mlockall
		// TODO munlockall
		// TODO mprotect
		30 => signal(regs),
		31 => kill(regs),
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
		32 => uname(regs),
		33 => reboot(regs),

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
