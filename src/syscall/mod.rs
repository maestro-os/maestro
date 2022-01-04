//! This module handles system calls. A system call is "function" that allows to communcate between
//! userspace and kernelspace.

mod _exit;
mod brk;
mod chdir;
mod chroot;
mod close;
mod creat;
mod delete_module;
mod dup2;
mod dup;
mod execve;
mod fchdir;
mod finit_module;
mod fork;
mod getcwd;
mod getegid;
mod geteuid;
mod getgid;
mod getpgid;
mod getpid;
mod getppid;
mod gettid;
mod getuid;
mod init_module;
mod ioctl;
mod kill;
mod mkdir;
mod mknod;
mod mmap;
mod modify_ldt;
mod mount;
mod msync;
mod munmap;
mod open;
mod pipe2;
mod pipe;
mod r#break;
mod read;
mod reboot;
mod set_thread_area;
mod set_tid_address;
mod setgid;
mod setpgid;
mod setuid;
mod signal;
mod sigreturn;
mod socketpair;
mod time;
mod umask;
mod umount;
mod uname;
mod unlink;
mod wait;
mod waitpid;
mod write;
mod writev;
pub mod util;

use crate::process::Process;
use crate::process::signal::Signal;
use crate::process;

//use modify_ldt::modify_ldt;
//use wait::wait;
use _exit::_exit;
use brk::brk;
use chdir::chdir;
use chroot::chroot;
use close::close;
use crate::process::Regs;
use creat::creat;
use delete_module::delete_module;
use dup2::dup2;
use dup::dup;
use execve::execve;
use fchdir::fchdir;
use finit_module::finit_module;
use fork::fork;
use getcwd::getcwd;
use getegid::getegid;
use geteuid::geteuid;
use getgid::getgid;
use getpgid::getpgid;
use getpid::getpid;
use getppid::getppid;
use gettid::gettid;
use getuid::getuid;
use init_module::init_module;
use ioctl::ioctl;
use kill::kill;
use mkdir::mkdir;
use mknod::mknod;
use mmap::mmap;
use mount::mount;
use msync::msync;
use munmap::munmap;
use open::open;
use pipe2::pipe2;
use pipe::pipe;
use r#break::r#break;
use read::read;
use reboot::reboot;
use set_thread_area::set_thread_area;
use set_tid_address::set_tid_address;
use setgid::setgid;
use setpgid::setpgid;
use setuid::setuid;
use signal::signal;
use sigreturn::sigreturn;
use socketpair::socketpair;
use time::time;
use umask::umask;
use umount::umount;
use uname::uname;
use unlink::unlink;
use waitpid::waitpid;
use write::write;
use writev::writev;

/// This function is called whenever a system call is triggered.
#[no_mangle]
pub extern "C" fn syscall_handler(regs: &mut Regs) {
	let id = regs.eax;

	let result = match id {
		// 0x000 => restart_syscall(regs),
		0x001 => _exit(regs),
		0x002 => fork(regs),
		0x003 => read(regs),
		0x004 => write(regs),
		0x005 => open(regs),
		0x006 => close(regs),
		0x007 => waitpid(regs),
		0x008 => creat(regs),
		// TODO 0x009 => link(regs),
		0x00a => unlink(regs),
		0x00b => execve(regs),
		0x00c => chdir(regs),
		0x00d => time(regs),
		0x00e => mknod(regs),
		// TODO 0x00f => chmod(regs),
		// TODO 0x010 => lchown(regs),
		0x011 => r#break(regs),
		// TODO 0x012 => oldstat(regs),
		// TODO 0x013 => lseek(regs),
		0x014 => getpid(regs),
		0x015 => mount(regs),
		0x016 => umount(regs),
		0x017 => setuid(regs),
		0x018 => getuid(regs),
		// TODO 0x019 => stime(regs),
		// TODO 0x01a => ptrace(regs),
		// TODO 0x01b => alarm(regs),
		// TODO 0x01c => oldfstat(regs),
		// TODO 0x01d => pause(regs),
		// TODO 0x01e => utime(regs),
		// TODO 0x01f => stty(regs),
		// TODO 0x020 => gtty(regs),
		// TODO 0x021 => access(regs),
		// TODO 0x022 => nice(regs),
		// TODO 0x023 => ftime(regs),
		// TODO 0x024 => sync(regs),
		0x025 => kill(regs),
		// TODO 0x026 => rename(regs),
		0x027 => mkdir(regs),
		// TODO 0x028 => rmdir(regs),
		0x029 => dup(regs),
		0x02a => pipe(regs),
		// TODO 0x02b => times(regs),
		// TODO 0x02c => prof(regs),
		0x02d => brk(regs),
		0x02e => setgid(regs),
		0x02f => getgid(regs),
		0x030 => signal(regs),
		0x031 => geteuid(regs),
		0x032 => getegid(regs),
		// TODO 0x033 => acct(regs),
		// TODO 0x034 => umount2(regs),
		// TODO 0x035 => lock(regs),
		0x036 => ioctl(regs),
		// TODO 0x037 => fcntl(regs),
		// TODO 0x038 => mpx(regs),
		0x039 => setpgid(regs),
		// TODO 0x03a => ulimit(regs),
		// TODO 0x03b => oldolduname(regs),
		0x03c => umask(regs),
		0x03d => chroot(regs),
		// TODO 0x03e => ustat(regs),
		0x03f => dup2(regs),
		0x040 => getppid(regs),
		// TODO 0x041 => getpgrp(regs),
		// TODO 0x042 => setsid(regs),
		// TODO 0x043 => sigaction(regs),
		// TODO 0x044 => sgetmask(regs),
		// TODO 0x045 => ssetmask(regs),
		// TODO 0x046 => setreuid(regs),
		// TODO 0x047 => setregid(regs),
		// TODO 0x048 => sigsuspend(regs),
		// TODO 0x049 => sigpending(regs),
		// TODO 0x04a => sethostname(regs),
		// TODO 0x04b => setrlimit(regs),
		// TODO 0x04c => getrlimit(regs),
		// TODO 0x04d => getrusage(regs),
		// TODO 0x04e => gettimeofday(regs),
		// TODO 0x04f => settimeofday(regs),
		// TODO 0x050 => getgroups(regs),
		// TODO 0x051 => setgroups(regs),
		// TODO 0x052 => select(regs),
		// TODO 0x053 => symlink(regs),
		// TODO 0x054 => oldlstat(regs),
		// TODO 0x055 => readlink(regs),
		// TODO 0x056 => uselib(regs),
		// TODO 0x057 => swapon(regs),
		0x058 => reboot(regs),
		// TODO 0x059 => readdir(regs),
		0x05a => mmap(regs),
		0x05b => munmap(regs),
		// TODO 0x05c => truncate(regs),
		// TODO 0x05d => ftruncate(regs),
		// TODO 0x05e => fchmod(regs),
		// TODO 0x05f => fchown(regs),
		// TODO 0x060 => getpriority(regs),
		// TODO 0x061 => setpriority(regs),
		// TODO 0x062 => profil(regs),
		// TODO 0x063 => statfs(regs),
		// TODO 0x064 => fstatfs(regs),
		// TODO 0x065 => ioperm(regs),
		// TODO 0x066 => socketcall(regs),
		// TODO 0x067 => syslog(regs),
		// TODO 0x068 => setitimer(regs),
		// TODO 0x069 => getitimer(regs),
		// TODO 0x06a => stat(regs),
		// TODO 0x06b => lstat(regs),
		// TODO 0x06c => fstat(regs),
		// TODO 0x06d => olduname(regs),
		// TODO 0x06e => iopl(regs),
		// TODO 0x06f => vhangup(regs),
		// TODO 0x070 => idle(regs),
		// TODO 0x071 => vm86old(regs),
		// TODO 0x072 => wait4(regs),
		// TODO 0x073 => swapoff(regs),
		// TODO 0x074 => sysinfo(regs),
		// TODO 0x075 => ipc(regs),
		// TODO 0x076 => fsync(regs),
		0x077 => sigreturn(regs),
		// TODO 0x078 => clone(regs),
		// TODO 0x079 => setdomainname(regs),
		0x07a => uname(regs),
		// 0x07b => modify_ldt(regs),
		// TODO 0x07c => adjtimex(regs),
		// TODO 0x07d => mprotect(regs),
		// TODO 0x07e => sigprocmask(regs),
		// TODO 0x07f => create_module(regs),
		0x080 => init_module(regs),
		0x081 => delete_module(regs),
		// TODO 0x082 => get_kernel_syms(regs),
		// TODO 0x083 => quotactl(regs),
		0x084 => getpgid(regs),
		0x085 => fchdir(regs),
		// TODO 0x086 => bdflush(regs),
		// TODO 0x087 => sysfs(regs),
		// TODO 0x088 => personality(regs),
		// TODO 0x089 => afs_syscall(regs),
		// TODO 0x08a => setfsuid(regs),
		// TODO 0x08b => setfsgid(regs),
		// TODO 0x08c => _llseek(regs),
		// TODO 0x08d => getdents(regs),
		// TODO 0x08e => _newselect(regs),
		// TODO 0x08f => flock(regs),
		0x090 => msync(regs),
		// TODO 0x091 => readv(regs),
		0x092 => writev(regs),
		// TODO 0x093 => getsid(regs),
		// TODO 0x094 => fdatasync(regs),
		// TODO 0x095 => _sysctl(regs),
		// TODO 0x096 => mlock(regs),
		// TODO 0x097 => munlock(regs),
		// TODO 0x098 => mlockall(regs),
		// TODO 0x099 => munlockall(regs),
		// TODO 0x09a => sched_setparam(regs),
		// TODO 0x09b => sched_getparam(regs),
		// TODO 0x09c => sched_setscheduler(regs),
		// TODO 0x09d => sched_getscheduler(regs),
		// TODO 0x09e => sched_yield(regs),
		// TODO 0x09f => sched_get_priority_max(regs),
		// TODO 0x0a0 => sched_get_priority_min(regs),
		// TODO 0x0a1 => sched_rr_get_interval(regs),
		// TODO 0x0a2 => nanosleep(regs),
		// TODO 0x0a3 => mremap(regs),
		// TODO 0x0a4 => setresuid(regs),
		// TODO 0x0a5 => getresuid(regs),
		// TODO 0x0a6 => vm86(regs),
		// TODO 0x0a7 => query_module(regs),
		// TODO 0x0a8 => poll(regs),
		// TODO 0x0a9 => nfsservctl(regs),
		// TODO 0x0aa => setresgid(regs),
		// TODO 0x0ab => getresgid(regs),
		// TODO 0x0ac => prctl(regs),
		// TODO 0x0ad => rt_sigreturn(regs),
		// TODO 0x0ae => rt_sigaction(regs),
		// TODO 0x0af => rt_sigprocmask(regs),
		// TODO 0x0b0 => rt_sigpending(regs),
		// TODO 0x0b1 => rt_sigtimedwait(regs),
		// TODO 0x0b2 => rt_sigqueueinfo(regs),
		// TODO 0x0b3 => rt_sigsuspend(regs),
		// TODO 0x0b4 => pread64(regs),
		// TODO 0x0b5 => pwrite64(regs),
		// TODO 0x0b6 => chown(regs),
		0x0b7 => getcwd(regs),
		// TODO 0x0b8 => capget(regs),
		// TODO 0x0b9 => capset(regs),
		// TODO 0x0ba => sigaltstack(regs),
		// TODO 0x0bb => sendfile(regs),
		// TODO 0x0bc => getpmsg(regs),
		// TODO 0x0bd => putpmsg(regs),
		// TODO 0x0be => vfork(regs),
		// TODO 0x0bf => ugetrlimit(regs),
		// TODO 0x0c0 => mmap2(regs),
		// TODO 0x0c1 => truncate64(regs),
		// TODO 0x0c2 => ftruncate64(regs),
		// TODO 0x0c3 => stat64(regs),
		// TODO 0x0c4 => lstat64(regs),
		// TODO 0x0c5 => fstat64(regs),
		// TODO 0x0c6 => lchown32(regs),
		// TODO 0x0c7 => getuid32(regs),
		// TODO 0x0c8 => getgid32(regs),
		// TODO 0x0c9 => geteuid32(regs),
		// TODO 0x0ca => getegid32(regs),
		// TODO 0x0cb => setreuid32(regs),
		// TODO 0x0cc => setregid32(regs),
		// TODO 0x0cd => getgroups32(regs),
		// TODO 0x0ce => setgroups32(regs),
		// TODO 0x0cf => fchown32(regs),
		// TODO 0x0d0 => setresuid32(regs),
		// TODO 0x0d1 => getresuid32(regs),
		// TODO 0x0d2 => setresgid32(regs),
		// TODO 0x0d3 => getresgid32(regs),
		// TODO 0x0d4 => chown32(regs),
		// TODO 0x0d5 => setuid32(regs),
		// TODO 0x0d6 => setgid32(regs),
		// TODO 0x0d7 => setfsuid32(regs),
		// TODO 0x0d8 => setfsgid32(regs),
		// TODO 0x0d9 => pivot_root(regs),
		// TODO 0x0da => mincore(regs),
		// TODO 0x0db => madvise(regs),
		// TODO 0x0dc => getdents64(regs),
		// TODO 0x0dd => fcntl64(regs),
		0x0e0 => gettid(regs),
		// TODO 0x0e1 => readahead(regs),
		// TODO 0x0e2 => setxattr(regs),
		// TODO 0x0e3 => lsetxattr(regs),
		// TODO 0x0e4 => fsetxattr(regs),
		// TODO 0x0e5 => getxattr(regs),
		// TODO 0x0e6 => lgetxattr(regs),
		// TODO 0x0e7 => fgetxattr(regs),
		// TODO 0x0e8 => listxattr(regs),
		// TODO 0x0e9 => llistxattr(regs),
		// TODO 0x0ea => flistxattr(regs),
		// TODO 0x0eb => removexattr(regs),
		// TODO 0x0ec => lremovexattr(regs),
		// TODO 0x0ed => fremovexattr(regs),
		// TODO 0x0ee => tkill(regs),
		// TODO 0x0ef => sendfile64(regs),
		// TODO 0x0f0 => futex(regs),
		// TODO 0x0f1 => sched_setaffinity(regs),
		// TODO 0x0f2 => sched_getaffinity(regs),
		0x0f3 => set_thread_area(regs),
		// TODO 0x0f4 => get_thread_area(regs),
		// TODO 0x0f5 => io_setup(regs),
		// TODO 0x0f6 => io_destroy(regs),
		// TODO 0x0f7 => io_getevents(regs),
		// TODO 0x0f8 => io_submit(regs),
		// TODO 0x0f9 => io_cancel(regs),
		// TODO 0x0fa => fadvise64(regs),
		// TODO 0x0fc => exit_group(regs),
		// TODO 0x0fd => lookup_dcookie(regs),
		// TODO 0x0fe => epoll_create(regs),
		// TODO 0x0ff => epoll_ctl(regs),
		// TODO 0x100 => epoll_wait(regs),
		// TODO 0x101 => remap_file_pages(regs),
		0x102 => set_tid_address(regs),
		// TODO 0x103 => timer_create(regs),
		// TODO 0x104 => timer_settime(regs),
		// TODO 0x105 => timer_gettime(regs),
		// TODO 0x106 => timer_getoverrun(regs),
		// TODO 0x107 => timer_delete(regs),
		// TODO 0x108 => clock_settime(regs),
		// TODO 0x109 => clock_gettime(regs),
		// TODO 0x10a => clock_getres(regs),
		// TODO 0x10b => clock_nanosleep(regs),
		// TODO 0x10c => statfs64(regs),
		// TODO 0x10d => fstatfs64(regs),
		// TODO 0x10e => tgkill(regs),
		// TODO 0x10f => utimes(regs),
		// TODO 0x110 => fadvise64_64(regs),
		// TODO 0x111 => vserver(regs),
		// TODO 0x112 => mbind(regs),
		// TODO 0x113 => get_mempolicy(regs),
		// TODO 0x114 => set_mempolicy(regs),
		// TODO 0x115 => mq_open(regs),
		// TODO 0x116 => mq_unlink(regs),
		// TODO 0x117 => mq_timedsend(regs),
		// TODO 0x118 => mq_timedreceive(regs),
		// TODO 0x119 => mq_notify(regs),
		// TODO 0x11a => mq_getsetattr(regs),
		// TODO 0x11b => kexec_load(regs),
		// TODO 0x11c => waitid(regs),
		// TODO 0x11e => add_key(regs),
		// TODO 0x11f => request_key(regs),
		// TODO 0x120 => keyctl(regs),
		// TODO 0x121 => ioprio_set(regs),
		// TODO 0x122 => ioprio_get(regs),
		// TODO 0x123 => inotify_init(regs),
		// TODO 0x124 => inotify_add_watch(regs),
		// TODO 0x125 => inotify_rm_watch(regs),
		// TODO 0x126 => migrate_pages(regs),
		// TODO 0x127 => openat(regs),
		// TODO 0x128 => mkdirat(regs),
		// TODO 0x129 => mknodat(regs),
		// TODO 0x12a => fchownat(regs),
		// TODO 0x12b => futimesat(regs),
		// TODO 0x12c => fstatat64(regs),
		// TODO 0x12d => unlinkat(regs),
		// TODO 0x12e => renameat(regs),
		// TODO 0x12f => linkat(regs),
		// TODO 0x130 => symlinkat(regs),
		// TODO 0x131 => readlinkat(regs),
		// TODO 0x132 => fchmodat(regs),
		// TODO 0x133 => faccessat(regs),
		// TODO 0x134 => pselect6(regs),
		// TODO 0x135 => ppoll(regs),
		// TODO 0x136 => unshare(regs),
		// TODO 0x137 => set_robust_list(regs),
		// TODO 0x138 => get_robust_list(regs),
		// TODO 0x139 => splice(regs),
		// TODO 0x13a => sync_file_range(regs),
		// TODO 0x13b => tee(regs),
		// TODO 0x13c => vmsplice(regs),
		// TODO 0x13d => move_pages(regs),
		// TODO 0x13e => getcpu(regs),
		// TODO 0x13f => epoll_pwait(regs),
		// TODO 0x140 => utimensat(regs),
		// TODO 0x141 => signalfd(regs),
		// TODO 0x142 => timerfd_create(regs),
		// TODO 0x143 => eventfd(regs),
		// TODO 0x144 => fallocate(regs),
		// TODO 0x145 => timerfd_settime(regs),
		// TODO 0x146 => timerfd_gettime(regs),
		// TODO 0x147 => signalfd4(regs),
		// TODO 0x148 => eventfd2(regs),
		// TODO 0x149 => epoll_create1(regs),
		// TODO 0x14a => dup3(regs),
		0x14b => pipe2(regs),
		// TODO 0x14c => inotify_init1(regs),
		// TODO 0x14d => preadv(regs),
		// TODO 0x14e => pwritev(regs),
		// TODO 0x14f => rt_tgsigqueueinfo(regs),
		// TODO 0x150 => perf_event_open(regs),
		// TODO 0x151 => recvmmsg(regs),
		// TODO 0x152 => fanotify_init(regs),
		// TODO 0x153 => fanotify_mark(regs),
		// TODO 0x154 => prlimit64(regs),
		// TODO 0x155 => name_to_handle_at(regs),
		// TODO 0x156 => open_by_handle_at(regs),
		// TODO 0x157 => clock_adjtime(regs),
		// TODO 0x158 => syncfs(regs),
		// TODO 0x159 => sendmmsg(regs),
		// TODO 0x15a => setns(regs),
		// TODO 0x15b => process_vm_readv(regs),
		// TODO 0x15c => process_vm_writev(regs),
		// TODO 0x15d => kcmp(regs),
		0x15e => finit_module(regs),
		// TODO 0x15f => sched_setattr(regs),
		// TODO 0x160 => sched_getattr(regs),
		// TODO 0x161 => renameat2(regs),
		// TODO 0x162 => seccomp(regs),
		// TODO 0x163 => getrandom(regs),
		// TODO 0x164 => memfd_create(regs),
		// TODO 0x165 => bpf(regs),
		// TODO 0x166 => execveat(regs),
		// TODO 0x167 => socket(regs),
		0x168 => socketpair(regs),
		// TODO 0x169 => bind(regs),
		// TODO 0x16a => connect(regs),
		// TODO 0x16b => listen(regs),
		// TODO 0x16c => accept4(regs),
		// TODO 0x16d => getsockopt(regs),
		// TODO 0x16e => setsockopt(regs),
		// TODO 0x16f => getsockname(regs),
		// TODO 0x170 => getpeername(regs),
		// TODO 0x171 => sendto(regs),
		// TODO 0x172 => sendmsg(regs),
		// TODO 0x173 => recvfrom(regs),
		// TODO 0x174 => recvmsg(regs),
		// TODO 0x175 => shutdown(regs),
		// TODO 0x176 => userfaultfd(regs),
		// TODO 0x177 => membarrier(regs),
		// TODO 0x178 => mlock2(regs),
		// TODO 0x179 => copy_file_range(regs),
		// TODO 0x17a => preadv2(regs),
		// TODO 0x17b => pwritev2(regs),
		// TODO 0x17c => pkey_mprotect(regs),
		// TODO 0x17d => pkey_alloc(regs),
		// TODO 0x17e => pkey_free(regs),
		// TODO 0x17f => statx(regs),
		// TODO 0x180 => arch_prctl(regs),

		// The system call doesn't exist. Killing the process with SIGSYS
		_ => {
			{
				let mutex = Process::get_current().unwrap();
				let mut guard = mutex.lock();
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
