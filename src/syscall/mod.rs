//! This module handles system calls. A system call is "function" that allows to communcate between
//! userspace and kernelspace.

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
mod msync;
mod munmap;
mod open;
mod pipe2;
mod pipe;
mod read;
mod reboot;
mod sbrk;
mod set_thread_area;
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
use crate::process::signal::Signal;
use crate::process;

use _exit::_exit;
use brk::brk;
use chdir::chdir;
use chroot::chroot;
use close::close;
use crate::process::Regs;
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
use msync::msync;
use munmap::munmap;
use open::open;
use pipe2::pipe2;
use pipe::pipe;
use read::read;
use reboot::reboot;
use sbrk::sbrk;
use set_thread_area::set_thread_area;
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
pub extern "C" fn syscall_handler(regs: &mut Regs) {
	let id = regs.eax;

	let result = match id {
		0x00 => open(regs),
		0x01 => umask(regs),
		0x02 => mkdir(regs),
		0x03 => pipe(regs),
		0x04 => pipe2(regs),
		0x05 => dup(regs),
		0x06 => dup2(regs),
		0x07 => close(regs),
		0x08 => unlink(regs),
		0x09 => chroot(regs),
		0x0a => getcwd(regs),
		0x0b => chdir(regs),
		0x0c => fchdir(regs),
		0x0d => read(regs),
		0x0e => write(regs),
		0x0f => _exit(regs),
		0x10 => fork(regs),
		0x11 => wait(regs),
		0x12 => waitpid(regs),
		0x13 => getuid(regs),
		0x14 => setuid(regs),
		0x15 => getgid(regs),
		0x16 => setgid(regs),
		0x17 => getpid(regs),
		0x18 => getppid(regs),
		0x19 => getpgid(regs),
		0x1a => setpgid(regs),
		0x1b => brk(regs),
		0x1c => sbrk(regs),
		0x1d => mmap(regs),
		0x1e => munmap(regs),
		0x1f => msync(regs),
		0x20 => signal(regs),
		0x21 => kill(regs),
		0x22 => socketpair(regs),
		0x23 => uname(regs),
		0x24 => reboot(regs),
		0x25 => init_module(regs),
		0x26 => finit_module(regs),
		0x27 => delete_module(regs),
		// TODO utime
		// TODO mknod
		// TODO link
		// TODO fcntl
		// TODO poll
		// TODO ppoll
		// TODO flock
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
		// TODO mount
		// TODO umount
		// TODO sync
		// TODO syncfs
		// TODO fsync
		// TODO fdatasync
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
		// TODO geteuid
		// TODO seteuid
		// TODO getegid
		// TODO setegid
		// TODO getsid
		// TODO setsid
		// TODO gettid
		// TODO mlock
		// TODO munlock
		// TODO mlockall
		// TODO munlockall
		// TODO mprotect
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
		// TODO pause
		0xf3 => set_thread_area(regs),

		0x200 => sigreturn(regs),

		// The system call doesn't exist. Killing the process with SIGSYS
		_ => {
		    {
			    let mut mutex = Process::get_current().unwrap();
			    let mut guard = mutex.lock(false);
			    let curr_proc = guard.get_mut();

			    // SIGSYS cannot be caught, thus the process will be terminated
			    curr_proc.kill(Signal::new(process::signal::SIGSYS).unwrap(), true);
		    }
			crate::enter_loop();
		}
	};

	// Setting the return value
	let retval = {
		if let Ok(val) = result {
			val as _
		} else {
			-result.unwrap_err() as _
		}
	};
	regs.eax = retval;
}
