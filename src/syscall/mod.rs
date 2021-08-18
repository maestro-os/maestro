//! This module handles system calls. A system call is "function" that allows to communcate between
//! userspace and kernelspace.
//! TODO doc

mod _exit;
mod brk;
mod chdir;
mod chroot;
mod close;
mod delete_module;
mod dup2;
mod dup;
mod fchdir;
mod finit_module;
mod fork;
mod getcwd;
mod getgid;
mod getpgid;
mod getpid;
mod getppid;
mod getuid;
mod init_module;
mod kill;
mod mkdir;
mod mmap;
mod munmap;
mod open;
mod pipe2;
mod pipe;
mod read;
mod reboot;
mod sbrk;
mod setgid;
mod setpgid;
mod setuid;
mod signal;
mod sigreturn;
mod socketpair;
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
use delete_module::delete_module;
use dup2::dup2;
use dup::dup;
use fchdir::fchdir;
use finit_module::finit_module;
use fork::fork;
use getcwd::getcwd;
use getgid::getgid;
use getpgid::getpgid;
use getpid::getpid;
use getppid::getppid;
use getuid::getuid;
use init_module::init_module;
use kill::kill;
use mkdir::mkdir;
use mmap::mmap;
use munmap::munmap;
use open::open;
use pipe2::pipe2;
use pipe::pipe;
use read::read;
use reboot::reboot;
use sbrk::sbrk;
use setgid::setgid;
use setpgid::setpgid;
use setuid::setuid;
use signal::signal;
use sigreturn::sigreturn;
use socketpair::socketpair;
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
		3 => mkdir(regs),
		// TODO mknod
		4 => pipe(regs),
		5 => pipe2(regs),
		// TODO link
		// TODO fcntl
		6 => dup(regs),
		7 => dup2(regs),
		// TODO poll
		// TODO ppoll
		// TODO flock
		8 => close(regs),
		9 => unlink(regs),
		10 => chroot(regs),
		11 => getcwd(regs),
		12 => chdir(regs),
		13 => fchdir(regs),
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
		14 => read(regs),
		15 => write(regs),
		// TODO mount
		// TODO umount
		// TODO sync
		// TODO syncfs
		// TODO fsync
		// TODO fdatasync
		16 => _exit(regs),
		17 => fork(regs),
		18 => wait(regs),
		19 => waitpid(regs),
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
		20 => getuid(regs),
		21 => setuid(regs),
		// TODO geteuid
		// TODO seteuid
		22 => getgid(regs),
		23 => setgid(regs),
		// TODO getegid
		// TODO setegid
		24 => getpid(regs),
		25 => getppid(regs),
		26 => getpgid(regs),
		27 => setpgid(regs),
		// TODO getsid
		// TODO setsid
		// TODO gettid
		28 => brk(regs),
		29 => sbrk(regs),
		30 => mmap(regs),
		31 => munmap(regs),
		// TODO mlock
		// TODO munlock
		// TODO mlockall
		// TODO munlockall
		// TODO mprotect
		32 => signal(regs),
		33 => kill(regs),
		// TODO pause
		34 => socketpair(regs),
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
		35 => uname(regs),
		36 => reboot(regs),
		37 => init_module(regs),
		38 => finit_module(regs),
		39 => delete_module(regs),

		512 => sigreturn(regs),

		// The system call doesn't exist. Killing the process with SIGSYS
		_ => {
			let mut mutex = Process::get_current().unwrap();
			let mut guard = mutex.lock(false);
			let curr_proc = guard.get_mut();

			// SIGSYS cannot be caught, thus the process will be terminated
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
