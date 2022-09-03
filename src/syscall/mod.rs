//! This module handles system calls. A system call is "function" that allows to communcate between
//! userspace and kernelspace.
//!
//! Documentation for each system call can be retrieved from the man. Type the command:
//! `man 2 <syscall>`

mod _exit;
mod _llseek;
mod _newselect;
mod access;
mod brk;
mod chdir;
mod chmod;
mod chroot;
mod clock_gettime64;
mod clock_gettime;
mod clone;
mod close;
mod creat;
mod delete_module;
mod dup2;
mod dup;
mod execve;
mod exit_group;
mod faccessat2;
mod faccessat;
mod fadvise64_64;
mod fchdir;
mod fchmod;
mod fchmodat;
mod fcntl64;
mod fcntl;
mod finit_module;
mod fork;
mod fstatfs64;
mod fstatfs;
mod getcwd;
mod getdents64;
mod getdents;
mod getegid32;
mod getegid;
mod geteuid32;
mod geteuid;
mod getgid32;
mod getgid;
mod getpgid;
mod getpid;
mod getppid;
mod getrandom;
mod getrusage;
mod gettid;
mod getuid32;
mod getuid;
mod init_module;
mod kill;
mod link;
mod linkat;
mod madvise;
mod mkdir;
mod mknod;
mod mmap2;
mod mmap;
mod modify_ldt;
mod mount;
mod msync;
mod munmap;
mod nanosleep;
mod open;
mod openat;
mod pipe2;
mod pipe;
mod poll;
mod prlimit64;
mod pselect6;
mod pwritev2;
mod pwritev;
mod r#break;
mod read;
mod readlink;
mod reboot;
mod renameat2;
mod rmdir;
mod rt_sigaction;
mod rt_sigprocmask;
mod select;
mod set_thread_area;
mod set_tid_address;
mod setgid32;
mod setgid;
mod sethostname;
mod setpgid;
mod setuid32;
mod setuid;
mod signal;
mod sigreturn;
mod socketpair;
mod statfs64;
mod statfs;
mod statx;
mod symlinkat;
mod syncfs;
mod time;
mod tkill;
mod truncate;
mod umask;
mod umount;
mod uname;
mod unlink;
mod unlinkat;
mod util;
mod vfork;
mod wait4;
mod wait;
mod waitpid;
mod write;
mod writev;
pub mod ioctl;

use crate::errno::Errno;
use crate::process::regs::Regs;
use crate::process::signal::Signal;
use crate::process::Process;

//use modify_ldt::modify_ldt;
//use wait::wait;
use _exit::_exit;
use _llseek::_llseek;
use _newselect::_newselect;
use access::access;
use brk::brk;
use chdir::chdir;
use chmod::chmod;
use chroot::chroot;
use clock_gettime64::clock_gettime64;
use clock_gettime::clock_gettime;
use clone::clone;
use close::close;
use creat::creat;
use delete_module::delete_module;
use dup2::dup2;
use dup::dup;
use execve::execve;
use exit_group::exit_group;
use faccessat2::faccessat2;
use faccessat::faccessat;
use fadvise64_64::fadvise64_64;
use fchdir::fchdir;
use fchmod::fchmod;
use fchmodat::fchmodat;
use fcntl64::fcntl64;
use fcntl::fcntl;
use finit_module::finit_module;
use fork::fork;
use fstatfs64::fstatfs64;
use fstatfs::fstatfs;
use getcwd::getcwd;
use getdents64::getdents64;
use getdents::getdents;
use getegid32::getegid32;
use getegid::getegid;
use geteuid32::geteuid32;
use geteuid::geteuid;
use getgid32::getgid32;
use getgid::getgid;
use getpgid::getpgid;
use getpid::getpid;
use getppid::getppid;
use getrandom::getrandom;
use getrusage::getrusage;
use gettid::gettid;
use getuid32::getuid32;
use getuid::getuid;
use init_module::init_module;
use ioctl::ioctl;
use kill::kill;
use link::link;
use linkat::linkat;
use madvise::madvise;
use mkdir::mkdir;
use mknod::mknod;
use mmap2::mmap2;
use mmap::mmap;
use mount::mount;
use msync::msync;
use munmap::munmap;
use nanosleep::nanosleep;
use open::open;
use openat::openat;
use pipe2::pipe2;
use pipe::pipe;
use poll::poll;
use prlimit64::prlimit64;
use pselect6::pselect6;
use pwritev2::pwritev2;
use pwritev::pwritev;
use r#break::r#break;
use read::read;
use readlink::readlink;
use reboot::reboot;
use renameat2::renameat2;
use rmdir::rmdir;
use rt_sigaction::rt_sigaction;
use rt_sigprocmask::rt_sigprocmask;
use select::select;
use set_thread_area::set_thread_area;
use set_tid_address::set_tid_address;
use setgid32::setgid32;
use setgid::setgid;
use sethostname::sethostname;
use setpgid::setpgid;
use setuid32::setuid32;
use setuid::setuid;
use signal::signal;
use sigreturn::sigreturn;
use socketpair::socketpair;
use statfs64::statfs64;
use statfs::statfs;
use statx::statx;
use symlinkat::symlinkat;
use syncfs::syncfs;
use time::time;
use tkill::tkill;
use truncate::truncate;
use umask::umask;
use umount::umount;
use uname::uname;
use unlink::unlink;
use unlinkat::unlinkat;
use vfork::vfork;
use wait4::wait4;
use waitpid::waitpid;
use write::write;
use writev::writev;

/// Structure representing a system call.
struct Syscall {
	/// The syscall's handler.
	pub handler: &'static dyn Fn(&Regs) -> Result<i32, Errno>,

	/// The syscall's name.
	pub name: &'static str,
}

/// Returns the system call associated with the given ID `id`.
/// If the syscall doesn't exist, the function returns None.
fn get_syscall(id: u32) -> Option<Syscall> {
	match id {
		0x001 => Some(Syscall {
			handler: &_exit,
			name: "_exit",
		}),
		0x002 => Some(Syscall {
			handler: &fork,
			name: "fork",
		}),
		0x003 => Some(Syscall {
			handler: &read,
			name: "read",
		}),
		0x004 => Some(Syscall {
			handler: &write,
			name: "write",
		}),
		0x005 => Some(Syscall {
			handler: &open,
			name: "open",
		}),
		0x006 => Some(Syscall {
			handler: &close,
			name: "close",
		}),
		0x007 => Some(Syscall {
			handler: &waitpid,
			name: "waitpid",
		}),
		0x008 => Some(Syscall {
			handler: &creat,
			name: "creat",
		}),
		0x009 => Some(Syscall {
			handler: &link,
			name: "link",
		}),
		0x00a => Some(Syscall {
			handler: &unlink,
			name: "unlink",
		}),
		0x00b => Some(Syscall {
			handler: &execve,
			name: "execve",
		}),
		0x00c => Some(Syscall {
			handler: &chdir,
			name: "chdir",
		}),
		0x00d => Some(Syscall {
			handler: &time,
			name: "time",
		}),
		0x00e => Some(Syscall {
			handler: &mknod,
			name: "mknod",
		}),
		0x00f => Some(Syscall {
			handler: &chmod,
			name: "chmod",
		}),
		// TODO 0x010 => Some(Syscall { handler: &lchown, name: "lchown" }),
		0x011 => Some(Syscall {
			handler: &r#break,
			name: "break",
		}),
		// TODO 0x012 => Some(Syscall { handler: &oldstat, name: "oldstat" }),
		// TODO 0x013 => Some(Syscall { handler: &lseek, name: "lseek" }),
		0x014 => Some(Syscall {
			handler: &getpid,
			name: "getpid",
		}),
		0x015 => Some(Syscall {
			handler: &mount,
			name: "mount",
		}),
		0x016 => Some(Syscall {
			handler: &umount,
			name: "umount",
		}),
		0x017 => Some(Syscall {
			handler: &setuid,
			name: "setuid",
		}),
		0x018 => Some(Syscall {
			handler: &getuid,
			name: "getuid",
		}),
		// TODO 0x019 => Some(Syscall { handler: &stime, name: "stime" }),
		// TODO 0x01a => Some(Syscall { handler: &ptrace, name: "ptrace" }),
		// TODO 0x01b => Some(Syscall { handler: &alarm, name: "alarm" }),
		// TODO 0x01c => Some(Syscall { handler: &oldfstat, name: "oldfstat" }),
		// TODO 0x01d => Some(Syscall { handler: &pause, name: "pause" }),
		// TODO 0x01e => Some(Syscall { handler: &utime, name: "utime" }),
		// TODO 0x01f => Some(Syscall { handler: &stty, name: "stty" }),
		// TODO 0x020 => Some(Syscall { handler: &gtty, name: "gtty" }),
		0x021 => Some(Syscall {
			handler: &access,
			name: "access",
		}),
		// TODO 0x022 => Some(Syscall { handler: &nice, name: "nice" }),
		// TODO 0x023 => Some(Syscall { handler: &ftime, name: "ftime" }),
		// TODO 0x024 => Some(Syscall { handler: &sync, name: "sync" }),
		0x025 => Some(Syscall {
			handler: &kill,
			name: "kill",
		}),
		// TODO 0x026 => Some(Syscall { handler: &rename, name: "rename" }),
		0x027 => Some(Syscall {
			handler: &mkdir,
			name: "mkdir",
		}),
		0x028 => Some(Syscall {
			handler: &rmdir,
			name: "rmdir",
		}),
		0x029 => Some(Syscall {
			handler: &dup,
			name: "dup",
		}),
		0x02a => Some(Syscall {
			handler: &pipe,
			name: "pipe",
		}),
		// TODO 0x02b => Some(Syscall { handler: &times, name: "times" }),
		// TODO 0x02c => Some(Syscall { handler: &prof, name: "prof" }),
		0x02d => Some(Syscall {
			handler: &brk,
			name: "brk",
		}),
		0x02e => Some(Syscall {
			handler: &setgid,
			name: "setgid",
		}),
		0x02f => Some(Syscall {
			handler: &getgid,
			name: "getgid",
		}),
		0x030 => Some(Syscall {
			handler: &signal,
			name: "signal",
		}),
		0x031 => Some(Syscall {
			handler: &geteuid,
			name: "geteuid",
		}),
		0x032 => Some(Syscall {
			handler: &getegid,
			name: "getegid",
		}),
		// TODO 0x033 => Some(Syscall { handler: &acct, name: "acct" }),
		// TODO 0x034 => Some(Syscall { handler: &umount2, name: "umount2" }),
		// TODO 0x035 => Some(Syscall { handler: &lock, name: "lock" }),
		0x036 => Some(Syscall {
			handler: &ioctl,
			name: "ioctl",
		}),
		0x037 => Some(Syscall {
			handler: &fcntl,
			name: "fcntl",
		}),
		// TODO 0x038 => Some(Syscall { handler: &mpx, name: "mpx" }),
		0x039 => Some(Syscall {
			handler: &setpgid,
			name: "setpgid",
		}),
		// TODO 0x03a => Some(Syscall { handler: &ulimit, name: "ulimit" }),
		// TODO 0x03b => Some(Syscall { handler: &oldolduname, name: "oldolduname" }),
		0x03c => Some(Syscall {
			handler: &umask,
			name: "umask",
		}),
		0x03d => Some(Syscall {
			handler: &chroot,
			name: "chroot",
		}),
		// TODO 0x03e => Some(Syscall { handler: &ustat, name: "ustat" }),
		0x03f => Some(Syscall {
			handler: &dup2,
			name: "dup2",
		}),
		0x040 => Some(Syscall {
			handler: &getppid,
			name: "getppid",
		}),
		// TODO 0x041 => Some(Syscall { handler: &getpgrp, name: "getpgrp" }),
		// TODO 0x042 => Some(Syscall { handler: &setsid, name: "setsid" }),
		// TODO 0x043 => Some(Syscall { handler: &sigaction, name: "sigaction" }),
		// TODO 0x044 => Some(Syscall { handler: &sgetmask, name: "sgetmask" }),
		// TODO 0x045 => Some(Syscall { handler: &ssetmask, name: "ssetmask" }),
		// TODO 0x046 => Some(Syscall { handler: &setreuid, name: "setreuid" }),
		// TODO 0x047 => Some(Syscall { handler: &setregid, name: "setregid" }),
		// TODO 0x048 => Some(Syscall { handler: &sigsuspend, name: "sigsuspend" }),
		// TODO 0x049 => Some(Syscall { handler: &sigpending, name: "sigpending" }),
		0x04a => Some(Syscall {
			handler: &sethostname,
			name: "sethostname",
		}),
		// TODO 0x04b => Some(Syscall { handler: &setrlimit, name: "setrlimit" }),
		// TODO 0x04c => Some(Syscall { handler: &getrlimit, name: "getrlimit" }),
		0x04d => Some(Syscall {
			handler: &getrusage,
			name: "getrusage",
		}),
		// TODO 0x04e => Some(Syscall { handler: &gettimeofday, name: "gettimeofday" }),
		// TODO 0x04f => Some(Syscall { handler: &settimeofday, name: "settimeofday" }),
		// TODO 0x050 => Some(Syscall { handler: &getgroups, name: "getgroups" }),
		// TODO 0x051 => Some(Syscall { handler: &setgroups, name: "setgroups" }),
		0x052 => Some(Syscall {
			handler: &select,
			name: "select",
		}),
		// TODO 0x053 => Some(Syscall { handler: &symlink, name: "symlink" }),
		// TODO 0x054 => Some(Syscall { handler: &oldlstat, name: "oldlstat" }),
		0x055 => Some(Syscall {
			handler: &readlink,
			name: "readlink",
		}),
		// TODO 0x056 => Some(Syscall { handler: &uselib, name: "uselib" }),
		// TODO 0x057 => Some(Syscall { handler: &swapon, name: "swapon" }),
		0x058 => Some(Syscall {
			handler: &reboot,
			name: "reboot",
		}),
		// TODO 0x059 => Some(Syscall { handler: &readdir, name: "readdir" }),
		0x05a => Some(Syscall {
			handler: &mmap,
			name: "mmap",
		}),
		0x05b => Some(Syscall {
			handler: &munmap,
			name: "munmap",
		}),
		0x05c => Some(Syscall {
			handler: &truncate,
			name: "truncate",
		}),
		// TODO 0x05d => Some(Syscall { handler: &ftruncate, name: "ftruncate" }),
		0x05e => Some(Syscall {
			handler: &fchmod,
			name: "fchmod",
		}),
		// TODO 0x05f => Some(Syscall { handler: &fchown, name: "fchown" }),
		// TODO 0x060 => Some(Syscall { handler: &getpriority, name: "getpriority" }),
		// TODO 0x061 => Some(Syscall { handler: &setpriority, name: "setpriority" }),
		// TODO 0x062 => Some(Syscall { handler: &profil, name: "profil" }),
		0x063 => Some(Syscall {
			handler: &statfs,
			name: "statfs",
		}),
		0x064 => Some(Syscall { handler: &fstatfs, name: "fstatfs" }),
		// TODO 0x065 => Some(Syscall { handler: &ioperm, name: "ioperm" }),
		// TODO 0x066 => Some(Syscall { handler: &socketcall, name: "socketcall" }),
		// TODO 0x067 => Some(Syscall { handler: &syslog, name: "syslog" }),
		// TODO 0x068 => Some(Syscall { handler: &setitimer, name: "setitimer" }),
		// TODO 0x069 => Some(Syscall { handler: &getitimer, name: "getitimer" }),
		// TODO 0x06a => Some(Syscall { handler: &stat, name: "stat" }),
		// TODO 0x06b => Some(Syscall { handler: &lstat, name: "lstat" }),
		// TODO 0x06c => Some(Syscall { handler: &fstat, name: "fstat" }),
		// TODO 0x06d => Some(Syscall { handler: &olduname, name: "olduname" }),
		// TODO 0x06e => Some(Syscall { handler: &iopl, name: "iopl" }),
		// TODO 0x06f => Some(Syscall { handler: &vhangup, name: "vhangup" }),
		// TODO 0x070 => Some(Syscall { handler: &idle, name: "idle" }),
		// TODO 0x071 => Some(Syscall { handler: &vm86old, name: "vm86old" }),
		0x072 => Some(Syscall {
			handler: &wait4,
			name: "wait4",
		}),
		// TODO 0x073 => Some(Syscall { handler: &swapoff, name: "swapoff" }),
		// TODO 0x074 => Some(Syscall { handler: &sysinfo, name: "sysinfo" }),
		// TODO 0x075 => Some(Syscall { handler: &ipc, name: "ipc" }),
		// TODO 0x076 => Some(Syscall { handler: &fsync, name: "fsync" }),
		0x077 => Some(Syscall {
			handler: &sigreturn,
			name: "sigreturn",
		}),
		0x078 => Some(Syscall {
			handler: &clone,
			name: "clone",
		}),
		// TODO 0x079 => Some(Syscall { handler: &setdomainname, name: "setdomainname" }),
		0x07a => Some(Syscall {
			handler: &uname,
			name: "uname",
		}),
		// TODO 0x07c => Some(Syscall { handler: &adjtimex, name: "adjtimex" }),
		// TODO 0x07d => Some(Syscall { handler: &mprotect, name: "mprotect" }),
		// TODO 0x07e => Some(Syscall { handler: &sigprocmask, name: "sigprocmask" }),
		// TODO 0x07f => Some(Syscall { handler: &create_module, name: "create_module" }),
		0x080 => Some(Syscall {
			handler: &init_module,
			name: "init_module",
		}),
		0x081 => Some(Syscall {
			handler: &delete_module,
			name: "delete_module",
		}),
		// TODO 0x083 => Some(Syscall { handler: &quotactl, name: "quotactl" }),
		0x084 => Some(Syscall {
			handler: &getpgid,
			name: "getpgid",
		}),
		0x085 => Some(Syscall {
			handler: &fchdir,
			name: "fchdir",
		}),
		// TODO 0x086 => Some(Syscall { handler: &bdflush, name: "bdflush" }),
		// TODO 0x087 => Some(Syscall { handler: &sysfs, name: "sysfs" }),
		// TODO 0x088 => Some(Syscall { handler: &personality, name: "personality" }),
		// TODO 0x089 => Some(Syscall { handler: &afs_syscall, name: "afs_syscall" }),
		// TODO 0x08a => Some(Syscall { handler: &setfsuid, name: "setfsuid" }),
		// TODO 0x08b => Some(Syscall { handler: &setfsgid, name: "setfsgid" }),
		0x08c => Some(Syscall {
			handler: &_llseek,
			name: "_llseek",
		}),
		0x08d => Some(Syscall {
			handler: &getdents,
			name: "getdents",
		}),
		0x08e => Some(Syscall {
			handler: &_newselect,
			name: "_newselect",
		}),
		// TODO 0x08f => Some(Syscall { handler: &flock, name: "flock" }),
		0x090 => Some(Syscall {
			handler: &msync,
			name: "msync",
		}),
		// TODO 0x091 => Some(Syscall { handler: &readv, name: "readv" }),
		0x092 => Some(Syscall {
			handler: &writev,
			name: "writev",
		}),
		// TODO 0x093 => Some(Syscall { handler: &getsid, name: "getsid" }),
		// TODO 0x094 => Some(Syscall { handler: &fdatasync, name: "fdatasync" }),
		// TODO 0x095 => Some(Syscall { handler: &_sysctl, name: "_sysctl" }),
		// TODO 0x096 => Some(Syscall { handler: &mlock, name: "mlock" }),
		// TODO 0x097 => Some(Syscall { handler: &munlock, name: "munlock" }),
		// TODO 0x098 => Some(Syscall { handler: &mlockall, name: "mlockall" }),
		// TODO 0x099 => Some(Syscall { handler: &munlockall, name: "munlockall" }),
		// TODO 0x09a => Some(Syscall { handler: &sched_setparam, name: "sched_setparam",
		//	args: &[] }),
		// TODO 0x09b => Some(Syscall { handler: &sched_getparam, name: "sched_getparam",
		//	args: &[] }),
		// TODO 0x09c => Some(Syscall { handler: &sched_setscheduler, name: "sched_setscheduler",
		//	args: &[] }),
		// TODO 0x09d => Some(Syscall { handler: &sched_getscheduler, name: "sched_getscheduler",
		//	args: &[] }),
		// TODO 0x09e => Some(Syscall { handler: &sched_yield, name: "sched_yield" }),
		// TODO 0x09f => Some(Syscall { handler: &sched_get_priority_max,
		//	name: "sched_get_priority_max" }),
		// TODO 0x0a0 => Some(Syscall { handler: &sched_get_priority_min,
		//	name: "sched_get_priority_min" }),
		// TODO 0x0a1 => Some(Syscall { handler: &sched_rr_get_interval,
		//	name: "sched_rr_get_interval" }),
		0x0a2 => Some(Syscall {
			handler: &nanosleep,
			name: "nanosleep",
		}),
		// TODO 0x0a3 => Some(Syscall { handler: &mremap, name: "mremap" }),
		// TODO 0x0a4 => Some(Syscall { handler: &setresuid, name: "setresuid" }),
		// TODO 0x0a5 => Some(Syscall { handler: &getresuid, name: "getresuid" }),
		// TODO 0x0a6 => Some(Syscall { handler: &vm86, name: "vm86" }),
		// TODO 0x0a7 => Some(Syscall { handler: &query_module, name: "query_module" }),
		0x0a8 => Some(Syscall {
			handler: &poll,
			name: "poll",
		}),
		// TODO 0x0a9 => Some(Syscall { handler: &nfsservctl, name: "nfsservctl" }),
		// TODO 0x0aa => Some(Syscall { handler: &setresgid, name: "setresgid" }),
		// TODO 0x0ab => Some(Syscall { handler: &getresgid, name: "getresgid" }),
		// TODO 0x0ac => Some(Syscall { handler: &prctl, name: "prctl" }),
		// TODO 0x0ad => Some(Syscall { handler: &rt_sigreturn, name: "rt_sigreturn" }),
		0x0ae => Some(Syscall {
			handler: &rt_sigaction,
			name: "rt_sigaction",
		}),
		0x0af => Some(Syscall {
			handler: &rt_sigprocmask,
			name: "rt_sigprocmask",
		}),
		// TODO 0x0b0 => Some(Syscall { handler: &rt_sigpending, name: "rt_sigpending",
		//	args: &[] }),
		// TODO 0x0b1 => Some(Syscall { handler: &rt_sigtimedwait, name: "rt_sigtimedwait",
		//	args: &[] }),
		// TODO 0x0b2 => Some(Syscall { handler: &rt_sigqueueinfo, name: "rt_sigqueueinfo",
		//	args: &[] }),
		// TODO 0x0b3 => Some(Syscall { handler: &rt_sigsuspend, name: "rt_sigsuspend",
		//	args: &[] }),
		// TODO 0x0b4 => Some(Syscall { handler: &pread64, name: "pread64" }),
		// TODO 0x0b5 => Some(Syscall { handler: &pwrite64, name: "pwrite64" }),
		// TODO 0x0b6 => Some(Syscall { handler: &chown, name: "chown" }),
		0x0b7 => Some(Syscall {
			handler: &getcwd,
			name: "getcwd",
		}),
		// TODO 0x0b8 => Some(Syscall { handler: &capget, name: "capget" }),
		// TODO 0x0b9 => Some(Syscall { handler: &capset, name: "capset" }),
		// TODO 0x0ba => Some(Syscall { handler: &sigaltstack, name: "sigaltstack" }),
		// TODO 0x0bb => Some(Syscall { handler: &sendfile, name: "sendfile" }),
		// TODO 0x0bc => Some(Syscall { handler: &getpmsg, name: "getpmsg" }),
		// TODO 0x0bd => Some(Syscall { handler: &putpmsg, name: "putpmsg" }),
		0x0be => Some(Syscall {
			handler: &vfork,
			name: "vfork",
		}),
		// TODO 0x0bf => Some(Syscall { handler: &ugetrlimit, name: "ugetrlimit" }),
		0x0c0 => Some(Syscall {
			handler: &mmap2,
			name: "mmap2",
		}),
		// TODO 0x0c1 => Some(Syscall { handler: &truncate64, name: "truncate64" }),
		// TODO 0x0c2 => Some(Syscall { handler: &ftruncate64, name: "ftruncate64" }),
		// TODO 0x0c3 => Some(Syscall { handler: &stat64, name: "stat64" }),
		// TODO 0x0c4 => Some(Syscall { handler: &lstat64, name: "lstat64" }),
		// TODO 0x0c5 => Some(Syscall { handler: &fstat64, name: "fstat64" }),
		// TODO 0x0c6 => Some(Syscall { handler: &lchown32, name: "lchown32" }),
		0x0c7 => Some(Syscall {
			handler: &getuid32,
			name: "getuid32",
		}),
		0x0c8 => Some(Syscall {
			handler: &getgid32,
			name: "getgid32",
		}),
		0x0c9 => Some(Syscall {
			handler: &geteuid32,
			name: "geteuid32",
		}),
		0x0ca => Some(Syscall {
			handler: &getegid32,
			name: "getegid32",
		}),
		// TODO 0x0cb => Some(Syscall { handler: &setreuid32, name: "setreuid32" }),
		// TODO 0x0cc => Some(Syscall { handler: &setregid32, name: "setregid32" }),
		// TODO 0x0cd => Some(Syscall { handler: &getgroups32, name: "getgroups32" }),
		// TODO 0x0ce => Some(Syscall { handler: &setgroups32, name: "setgroups32" }),
		// TODO 0x0cf => Some(Syscall { handler: &fchown32, name: "fchown32" }),
		// TODO 0x0d0 => Some(Syscall { handler: &setresuid32, name: "setresuid32" }),
		// TODO 0x0d1 => Some(Syscall { handler: &getresuid32, name: "getresuid32" }),
		// TODO 0x0d2 => Some(Syscall { handler: &setresgid32, name: "setresgid32" }),
		// TODO 0x0d3 => Some(Syscall { handler: &getresgid32, name: "getresgid32" }),
		// TODO 0x0d4 => Some(Syscall { handler: &chown32, name: "chown32" }),
		0x0d5 => Some(Syscall {
			handler: &setuid32,
			name: "setuid32",
		}),
		0x0d6 => Some(Syscall {
			handler: &setgid32,
			name: "setgid32",
		}),
		// TODO 0x0d7 => Some(Syscall { handler: &setfsuid32, name: "setfsuid32" }),
		// TODO 0x0d8 => Some(Syscall { handler: &setfsgid32, name: "setfsgid32" }),
		// TODO 0x0d9 => Some(Syscall { handler: &pivot_root, name: "pivot_root" }),
		// TODO 0x0da => Some(Syscall { handler: &mincore, name: "mincore" }),
		0x0db => Some(Syscall {
			handler: &madvise,
			name: "madvise",
		}),
		0x0dc => Some(Syscall {
			handler: &getdents64,
			name: "getdents64",
		}),
		0x0dd => Some(Syscall {
			handler: &fcntl64,
			name: "fcntl64",
		}),
		0x0e0 => Some(Syscall {
			handler: &gettid,
			name: "gettid",
		}),
		// TODO 0x0e1 => Some(Syscall { handler: &readahead, name: "readahead" }),
		// TODO 0x0e2 => Some(Syscall { handler: &setxattr, name: "setxattr" }),
		// TODO 0x0e3 => Some(Syscall { handler: &lsetxattr, name: "lsetxattr" }),
		// TODO 0x0e4 => Some(Syscall { handler: &fsetxattr, name: "fsetxattr" }),
		// TODO 0x0e5 => Some(Syscall { handler: &getxattr, name: "getxattr" }),
		// TODO 0x0e6 => Some(Syscall { handler: &lgetxattr, name: "lgetxattr" }),
		// TODO 0x0e7 => Some(Syscall { handler: &fgetxattr, name: "fgetxattr" }),
		// TODO 0x0e8 => Some(Syscall { handler: &listxattr, name: "listxattr" }),
		// TODO 0x0e9 => Some(Syscall { handler: &llistxattr, name: "llistxattr" }),
		// TODO 0x0ea => Some(Syscall { handler: &flistxattr, name: "flistxattr" }),
		// TODO 0x0eb => Some(Syscall { handler: &removexattr, name: "removexattr" }),
		// TODO 0x0ec => Some(Syscall { handler: &lremovexattr, name: "lremovexattr" }),
		// TODO 0x0ed => Some(Syscall { handler: &fremovexattr, name: "fremovexattr" }),
		0x0ee => Some(Syscall {
			handler: &tkill,
			name: "tkill",
		}),
		// TODO 0x0ef => Some(Syscall { handler: &sendfile64, name: "sendfile64" }),
		// TODO 0x0f0 => Some(Syscall { handler: &futex, name: "futex" }),
		// TODO 0x0f1 => Some(Syscall { handler: &sched_setaffinity, name: "sched_setaffinity",
		//	args: &[] }),
		// TODO 0x0f2 => Some(Syscall { handler: &sched_getaffinity, name: "sched_getaffinity",
		//	args: &[] }),
		0x0f3 => Some(Syscall {
			handler: &set_thread_area,
			name: "set_thread_area",
		}),
		// TODO 0x0f4 => Some(Syscall { handler: &get_thread_area, name: "get_thread_area",
		//	args: &[] }),
		// TODO 0x0f5 => Some(Syscall { handler: &io_setup, name: "io_setup" }),
		// TODO 0x0f6 => Some(Syscall { handler: &io_destroy, name: "io_destroy" }),
		// TODO 0x0f7 => Some(Syscall { handler: &io_getevents, name: "io_getevents" }),
		// TODO 0x0f8 => Some(Syscall { handler: &io_submit, name: "io_submit" }),
		// TODO 0x0f9 => Some(Syscall { handler: &io_cancel, name: "io_cancel" }),
		// TODO 0x0fa => Some(Syscall { handler: &fadvise64, name: "fadvise64" }),
		0x0fc => Some(Syscall {
			handler: &exit_group,
			name: "exit_group",
		}),
		// TODO 0x0fd => Some(Syscall { handler: &lookup_dcookie, name: "lookup_dcookie",
		//	args: &[] }),
		// TODO 0x0fe => Some(Syscall { handler: &epoll_create, name: "epoll_create" }),
		// TODO 0x0ff => Some(Syscall { handler: &epoll_ctl, name: "epoll_ctl" }),
		// TODO 0x100 => Some(Syscall { handler: &epoll_wait, name: "epoll_wait" }),
		// TODO 0x101 => Some(Syscall { handler: &remap_file_pages, name: "remap_file_pages",
		//	args: &[] }),
		0x102 => Some(Syscall {
			handler: &set_tid_address,
			name: "set_tid_address",
		}),
		// TODO 0x103 => Some(Syscall { handler: &timer_create, name: "timer_create" }),
		// TODO 0x104 => Some(Syscall { handler: &timer_settime, name: "timer_settime",
		//	args: &[] }),
		// TODO 0x105 => Some(Syscall { handler: &timer_gettime, name: "timer_gettime",
		//	args: &[] }),
		// TODO 0x106 => Some(Syscall { handler: &timer_getoverrun, name: "timer_getoverrun",
		//	args: &[] }),
		// TODO 0x107 => Some(Syscall { handler: &timer_delete, name: "timer_delete" }),
		// TODO 0x108 => Some(Syscall { handler: &clock_settime, name: "clock_settime",
		//	args: &[] }),
		0x109 => Some(Syscall {
			handler: &clock_gettime,
			name: "clock_gettime",
		}),
		// TODO 0x10a => Some(Syscall { handler: &clock_getres, name: "clock_getres" }),
		// TODO 0x10b => Some(Syscall { handler: &clock_nanosleep, name: "clock_nanosleep",
		//	args: &[] }),
		0x10c => Some(Syscall {
			handler: &statfs64,
			name: "statfs64",
		}),
		0x10d => Some(Syscall { handler: &fstatfs64, name: "fstatfs64" }),
		// TODO 0x10e => Some(Syscall { handler: &tgkill, name: "tgkill" }),
		// TODO 0x10f => Some(Syscall { handler: &utimes, name: "utimes" }),
		0x110 => Some(Syscall {
			handler: &fadvise64_64,
			name: "fadvise64_64",
		}),
		// TODO 0x111 => Some(Syscall { handler: &vserver, name: "vserver" }),
		// TODO 0x112 => Some(Syscall { handler: &mbind, name: "mbind" }),
		// TODO 0x113 => Some(Syscall { handler: &get_mempolicy, name: "get_mempolicy",
		//	args: &[] }),
		// TODO 0x114 => Some(Syscall { handler: &set_mempolicy, name: "set_mempolicy",
		//	args: &[] }),
		// TODO 0x115 => Some(Syscall { handler: &mq_open, name: "mq_open" }),
		// TODO 0x116 => Some(Syscall { handler: &mq_unlink, name: "mq_unlink" }),
		// TODO 0x117 => Some(Syscall { handler: &mq_timedsend, name: "mq_timedsend" }),
		// TODO 0x118 => Some(Syscall { handler: &mq_timedreceive, name: "mq_timedreceive",
		//	args: &[] }),
		// TODO 0x119 => Some(Syscall { handler: &mq_notify, name: "mq_notify" }),
		// TODO 0x11a => Some(Syscall { handler: &mq_getsetattr, name: "mq_getsetattr",
		//	args: &[] }),
		// TODO 0x11b => Some(Syscall { handler: &kexec_load, name: "kexec_load" }),
		// TODO 0x11c => Some(Syscall { handler: &waitid, name: "waitid" }),
		// TODO 0x11e => Some(Syscall { handler: &add_key, name: "add_key" }),
		// TODO 0x11f => Some(Syscall { handler: &request_key, name: "request_key" }),
		// TODO 0x120 => Some(Syscall { handler: &keyctl, name: "keyctl" }),
		// TODO 0x121 => Some(Syscall { handler: &ioprio_set, name: "ioprio_set" }),
		// TODO 0x122 => Some(Syscall { handler: &ioprio_get, name: "ioprio_get" }),
		// TODO 0x123 => Some(Syscall { handler: &inotify_init, name: "inotify_init" }),
		// TODO 0x124 => Some(Syscall { handler: &inotify_add_watch, name: "inotify_add_watch",
		//	args: &[] }),
		// TODO 0x125 => Some(Syscall { handler: &inotify_rm_watch, name: "inotify_rm_watch",
		//	args: &[] }),
		// TODO 0x126 => Some(Syscall { handler: &migrate_pages, name: "migrate_pages",
		//	args: &[] }),
		0x127 => Some(Syscall {
			handler: &openat,
			name: "openat",
		}),
		// TODO 0x128 => Some(Syscall { handler: &mkdirat, name: "mkdirat" }),
		// TODO 0x129 => Some(Syscall { handler: &mknodat, name: "mknodat" }),
		// TODO 0x12a => Some(Syscall { handler: &fchownat, name: "fchownat" }),
		// TODO 0x12b => Some(Syscall { handler: &futimesat, name: "futimesat" }),
		// TODO 0x12c => Some(Syscall { handler: &fstatat64, name: "fstatat64" }),
		0x12d => Some(Syscall {
			handler: &unlinkat,
			name: "unlinkat",
		}),
		// TODO 0x12e => Some(Syscall { handler: &renameat, name: "renameat" }),
		0x12f => Some(Syscall {
			handler: &linkat,
			name: "linkat",
		}),
		0x130 => Some(Syscall {
			handler: &symlinkat,
			name: "symlinkat",
		}),
		// TODO 0x131 => Some(Syscall { handler: &readlinkat, name: "readlinkat" }),
		0x132 => Some(Syscall {
			handler: &fchmodat,
			name: "fchmodat",
		}),
		0x133 => Some(Syscall {
			handler: &faccessat,
			name: "faccessat",
		}),
		0x134 => Some(Syscall {
			handler: &pselect6,
			name: "pselect6",
		}),
		// TODO 0x135 => Some(Syscall { handler: &ppoll, name: "ppoll" }),
		// TODO 0x136 => Some(Syscall { handler: &unshare, name: "unshare" }),
		// TODO 0x137 => Some(Syscall { handler: &set_robust_list, name: "set_robust_list",
		//	args: &[] }),
		// TODO 0x138 => Some(Syscall { handler: &get_robust_list, name: "get_robust_list",
		//	args: &[] }),
		// TODO 0x139 => Some(Syscall { handler: &splice, name: "splice" }),
		// TODO 0x13a => Some(Syscall { handler: &sync_file_range, name: "sync_file_range",
		//	args: &[] }),
		// TODO 0x13b => Some(Syscall { handler: &tee, name: "tee" }),
		// TODO 0x13c => Some(Syscall { handler: &vmsplice, name: "vmsplice" }),
		// TODO 0x13d => Some(Syscall { handler: &move_pages, name: "move_pages" }),
		// TODO 0x13e => Some(Syscall { handler: &getcpu, name: "getcpu" }),
		// TODO 0x13f => Some(Syscall { handler: &epoll_pwait, name: "epoll_pwait" }),
		// TODO 0x140 => Some(Syscall { handler: &utimensat, name: "utimensat" }),
		// TODO 0x141 => Some(Syscall { handler: &signalfd, name: "signalfd" }),
		// TODO 0x142 => Some(Syscall { handler: &timerfd_create, name: "timerfd_create",
		//	args: &[] }),
		// TODO 0x143 => Some(Syscall { handler: &eventfd, name: "eventfd" }),
		// TODO 0x144 => Some(Syscall { handler: &fallocate, name: "fallocate" }),
		// TODO 0x145 => Some(Syscall { handler: &timerfd_settime, name: "timerfd_settime",
		//	args: &[] }),
		// TODO 0x146 => Some(Syscall { handler: &timerfd_gettime, name: "timerfd_gettime",
		//	args: &[] }),
		// TODO 0x147 => Some(Syscall { handler: &signalfd4, name: "signalfd4" }),
		// TODO 0x148 => Some(Syscall { handler: &eventfd2, name: "eventfd2" }),
		// TODO 0x149 => Some(Syscall { handler: &epoll_create1, name: "epoll_create1",
		//	args: &[] }),
		// TODO 0x14a => Some(Syscall { handler: &dup3, name: "dup3" }),
		0x14b => Some(Syscall {
			handler: &pipe2,
			name: "pipe2",
		}),
		// TODO 0x14c => Some(Syscall { handler: &inotify_init1, name: "inotify_init1",
		//	args: &[] }),
		// TODO 0x14d => Some(Syscall { handler: &preadv, name: "preadv" }),
		0x14e => Some(Syscall {
			handler: &pwritev,
			name: "pwritev",
		}),
		// TODO 0x14f => Some(Syscall { handler: &rt_tgsigqueueinfo, name: "rt_tgsigqueueinfo",
		//	args: &[] }),
		// TODO 0x150 => Some(Syscall { handler: &perf_event_open, name: "perf_event_open",
		//	args: &[] }),
		// TODO 0x151 => Some(Syscall { handler: &recvmmsg, name: "recvmmsg" }),
		// TODO 0x152 => Some(Syscall { handler: &fanotify_init, name: "fanotify_init",
		//	args: &[] }),
		// TODO 0x153 => Some(Syscall { handler: &fanotify_mark, name: "fanotify_mark",
		//	args: &[] }),
		0x154 => Some(Syscall {
			handler: &prlimit64,
			name: "prlimit64",
		}),
		// TODO 0x155 => Some(Syscall { handler: &name_to_handle_at, name: "name_to_handle_at",
		//	args: &[] }),
		// TODO 0x156 => Some(Syscall { handler: &open_by_handle_at, name: "open_by_handle_at",
		//	args: &[] }),
		// TODO 0x157 => Some(Syscall { handler: &clock_adjtime, name: "clock_adjtime",
		//	args: &[] }),
		0x158 => Some(Syscall {
			handler: &syncfs,
			name: "syncfs",
		}),
		// TODO 0x159 => Some(Syscall { handler: &sendmmsg, name: "sendmmsg" }),
		// TODO 0x15a => Some(Syscall { handler: &setns, name: "setns" }),
		// TODO 0x15b => Some(Syscall { handler: &process_vm_readv, name: "process_vm_readv",
		//	args: &[] }),
		// TODO 0x15c => Some(Syscall { handler: &process_vm_writev, name: "process_vm_writev",
		//	args: &[] }),
		// TODO 0x15d => Some(Syscall { handler: &kcmp, name: "kcmp" }),
		0x15e => Some(Syscall {
			handler: &finit_module,
			name: "finit_module",
		}),
		// TODO 0x15f => Some(Syscall { handler: &sched_setattr, name: "sched_setattr",
		//	args: &[] }),
		// TODO 0x160 => Some(Syscall { handler: &sched_getattr, name: "sched_getattr",
		//	args: &[] }),
		0x161 => Some(Syscall {
			handler: &renameat2,
			name: "renameat2",
		}),
		// TODO 0x162 => Some(Syscall { handler: &seccomp, name: "seccomp" }),
		0x163 => Some(Syscall {
			handler: &getrandom,
			name: "getrandom",
		}),
		// TODO 0x164 => Some(Syscall { handler: &memfd_create, name: "memfd_create",
		//	args: &[] }),
		// TODO 0x165 => Some(Syscall { handler: &bpf, name: "bpf" }),
		// TODO 0x166 => Some(Syscall { handler: &execveat, name: "execveat" }),
		// TODO 0x167 => Some(Syscall { handler: &socket, name: "socket" }),
		0x168 => Some(Syscall {
			handler: &socketpair,
			name: "socketpair",
		}),
		// TODO 0x169 => Some(Syscall { handler: &bind, name: "bind" }),
		// TODO 0x16a => Some(Syscall { handler: &connect, name: "connect" }),
		// TODO 0x16b => Some(Syscall { handler: &listen, name: "listen" }),
		// TODO 0x16c => Some(Syscall { handler: &accept4, name: "accept4" }),
		// TODO 0x16d => Some(Syscall { handler: &getsockopt, name: "getsockopt" }),
		// TODO 0x16e => Some(Syscall { handler: &setsockopt, name: "setsockopt" }),
		// TODO 0x16f => Some(Syscall { handler: &getsockname, name: "getsockname" }),
		// TODO 0x170 => Some(Syscall { handler: &getpeername, name: "getpeername" }),
		// TODO 0x171 => Some(Syscall { handler: &sendto, name: "sendto" }),
		// TODO 0x172 => Some(Syscall { handler: &sendmsg, name: "sendmsg" }),
		// TODO 0x173 => Some(Syscall { handler: &recvfrom, name: "recvfrom" }),
		// TODO 0x174 => Some(Syscall { handler: &recvmsg, name: "recvmsg" }),
		// TODO 0x175 => Some(Syscall { handler: &shutdown, name: "shutdown" }),
		// TODO 0x176 => Some(Syscall { handler: &userfaultfd, name: "userfaultfd" }),
		// TODO 0x177 => Some(Syscall { handler: &membarrier, name: "membarrier" }),
		// TODO 0x178 => Some(Syscall { handler: &mlock2, name: "mlock2" }),
		// TODO 0x179 => Some(Syscall { handler: &copy_file_range, name: "copy_file_range",
		//	args: &[] }),
		// TODO 0x17a => Some(Syscall { handler: &preadv2, name: "preadv2" }),
		0x17b => Some(Syscall {
			handler: &pwritev2,
			name: "pwritev2",
		}),
		// TODO 0x17c => Some(Syscall { handler: &pkey_mprotect, name: "pkey_mprotect",
		//	args: &[] }),
		// TODO 0x17d => Some(Syscall { handler: &pkey_alloc, name: "pkey_alloc" }),
		// TODO 0x17e => Some(Syscall { handler: &pkey_free, name: "pkey_free" }),
		0x17f => Some(Syscall {
			handler: &statx,
			name: "statx",
		}),
		// TODO 0x180 => Some(Syscall { handler: &arch_prctl, name: "arch_prctl" }),
		// TODO 0x181 => Some(Syscall { handler: &io_pgetevents, name: "io_pgetevents",
		//	args: &[] }),
		// TODO 0x182 => Some(Syscall { handler: &rseq, name: "rseq" }),
		// TODO 0x189 => Some(Syscall { handler: &semget, name: "semget" }),
		// TODO 0x18a => Some(Syscall { handler: &semctl, name: "semctl" }),
		// TODO 0x18b => Some(Syscall { handler: &shmget, name: "shmget" }),
		// TODO 0x18c => Some(Syscall { handler: &shmctl, name: "shmctl" }),
		// TODO 0x18d => Some(Syscall { handler: &shmat, name: "shmat" }),
		// TODO 0x18e => Some(Syscall { handler: &shmdt, name: "shmdt" }),
		// TODO 0x18f => Some(Syscall { handler: &msgget, name: "msgget" }),
		// TODO 0x190 => Some(Syscall { handler: &msgsnd, name: "msgsnd" }),
		// TODO 0x191 => Some(Syscall { handler: &msgrcv, name: "msgrcv" }),
		// TODO 0x192 => Some(Syscall { handler: &msgctl, name: "msgctl" }),
		0x193 => Some(Syscall {
			handler: &clock_gettime64,
			name: "clock_gettime64",
		}),
		// TODO 0x194 => Some(Syscall { handler: &clock_settime64, name: "clock_settime64" }),
		// TODO 0x195 => Some(Syscall { handler: &clock_adjtime64, name: "clock_adjtime64" }),
		// TODO 0x196 => Some(Syscall { handler: &clock_getres_time64,
		//	name: "clock_getres_time64" }),
		// TODO 0x197 => Some(Syscall { handler: &clock_nanosleep_time64,
		//	name: "clock_nanosleep_time64" }),
		// TODO 0x198 => Some(Syscall { handler: &timer_gettime64, name: "timer_gettime64" }),
		// TODO 0x199 => Some(Syscall { handler: &timer_settime64, name: "timer_settime64" }),
		// TODO 0x19a => Some(Syscall { handler: &timerfd_gettime64, name: "timerfd_gettime64" }),
		// TODO 0x19b => Some(Syscall { handler: &timerfd_settime64, name: "timerfd_settime64" }),
		// TODO 0x19c => Some(Syscall { handler: &utimensat_time64, name: "utimensat_time64" }),
		// TODO 0x19d => Some(Syscall { handler: &pselect6_time64, name: "pselect6_time64" }),
		// TODO 0x19e => Some(Syscall { handler: &ppoll_time64, name: "ppoll_time64" }),
		// TODO 0x1a0 => Some(Syscall { handler: &io_pgetevents_time64,
		//	name: "io_pgetevents_time64" }),
		// TODO 0x1a1 => Some(Syscall { handler: &recvmmsg_time64, name: "recvmmsg_time64" }),
		// TODO 0x1a2 => Some(Syscall { handler: &mq_timedsend_time64,
		//	name: "mq_timedsend_time64" }),
		// TODO 0x1a3 => Some(Syscall { handler: &mq_timedreceive_time64,
		//	name: "mq_timedreceive_time64" }),
		// TODO 0x1a4 => Some(Syscall { handler: &semtimedop_time64, name: "semtimedop_time64" }),
		// TODO 0x1a5 => Some(Syscall { handler: &rt_sigtimedwait_time64,
		//	name: "rt_sigtimedwait_time64" }),
		// TODO 0x1a6 => Some(Syscall { handler: &futex_time64, name: "futex_time64" }),
		// TODO 0x1a7 => Some(Syscall { handler: &sched_rr_get_interval_time64,
		//	name: "sched_rr_get_interval_time64" }),
		// TODO 0x1a8 => Some(Syscall { handler: &pidfd_send_signal, name: "pidfd_send_signal" }),
		// TODO 0x1a9 => Some(Syscall { handler: &io_uring_setup, name: "io_uring_setup" }),
		// TODO 0x1aa => Some(Syscall { handler: &io_uring_enter, name: "io_uring_enter" }),
		// TODO 0x1ab => Some(Syscall { handler: &io_uring_register, name: "io_uring_register" }),
		// TODO 0x1ac => Some(Syscall { handler: &open_tree, name: "open_tree" }),
		// TODO 0x1ad => Some(Syscall { handler: &move_mount, name: "move_mount" }),
		// TODO 0x1ae => Some(Syscall { handler: &fsopen, name: "fsopen" }),
		// TODO 0x1af => Some(Syscall { handler: &fsconfig, name: "fsconfig" }),
		// TODO 0x1b0 => Some(Syscall { handler: &fsmount, name: "fsmount" }),
		// TODO 0x1b1 => Some(Syscall { handler: &fspick, name: "fspick" }),
		// TODO 0x1b2 => Some(Syscall { handler: &pidfd_open, name: "pidfd_open" }),
		// TODO 0x1b3 => Some(Syscall { handler: &clone3, name: "clone3" }),
		// TODO 0x1b4 => Some(Syscall { handler: &close_range, name: "close_range" }),
		// TODO 0x1b5 => Some(Syscall { handler: &openat2, name: "openat2" }),
		// TODO 0x1b6 => Some(Syscall { handler: &pidfd_getfd, name: "pidfd_getfd" }),
		0x1b7 => Some(Syscall {
			handler: &faccessat2,
			name: "faccessat2",
		}),
		// TODO 0x1b8 => Some(Syscall { handler: &process_madvise, name: "process_madvise" }),
		// TODO 0x1b9 => Some(Syscall { handler: &epoll_pwait2, name: "epoll_pwait2" }),
		// TODO 0x1ba => Some(Syscall { handler: &mount_setattr, name: "mount_setattr" }),
		// TODO 0x1bb => Some(Syscall { handler: &quotactl_fd, name: "quotactl_fd" }),
		// TODO 0x1bc => Some(Syscall { handler: &landlock_create_ruleset,
		//	name: "landlock_create_ruleset" }),
		// TODO 0x1bd => Some(Syscall { handler: &landlock_add_rule, name: "landlock_add_rule" }),
		// TODO 0x1be => Some(Syscall { handler: &landlock_restrict_self,
		//	name: "landlock_restrict_self" }),
		// TODO 0x1bf => Some(Syscall { handler: &memfd_secret, name: "memfd_secret" }),
		// TODO 0x1c0 => Some(Syscall { handler: &process_mrelease, name: "process_mrelease" }),
		// TODO 0x1c1 => Some(Syscall { handler: &futex_waitv, name: "futex_waitv" }),
		// TODO 0x1c2 => Some(Syscall { handler: &set_mempolicy_home_node,
		//	name: "set_mempolicy_home_node" }),
		_ => None,
	}
}

/// Prints the trace for a syscall.
/// `regs` are the registers passed to the syscall.
/// `result` is the result of the syscall.
fn print_strace(regs: &Regs, result: Option<Result<i32, Errno>>) {
	let pid = {
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();
		proc.get_pid()
	};

	// Getting syscall name
	let id = regs.eax;

	// TODO Optimize (holes in the syscall table)
	let syscall = match get_syscall(id) {
		Some(syscall) => syscall,
		None => {
			println!("invalid syscall (pid: {}): {:x}", pid, id);
			return;
		}
	};

	if let Some(result) = result {
		match result {
			Ok(val) => println!(
				"strace end (pid: {}): {} -> Ok(0x{:x})",
				pid, syscall.name, val as usize
			),
			Err(errno) => println!(
				"strace end (pid: {}): {} -> Errno({})",
				pid, syscall.name, errno
			),
		}
	} else {
		println!("strace start (pid: {}): {}", pid, syscall.name);

		// TODO Make everything print at once (becomes unreadable when several processes are
		// running)
		/*for i in 0..syscall.args.len() {
			let val = match i {
				0 => regs.ebx,
				1 => regs.ecx,
				2 => regs.edx,
				3 => regs.esi,
				4 => regs.edi,
				5 => regs.ebp,

				_ => 0,
			};

			if i + 1 < syscall.args.len() {
				print!("{} = 0x{:x}, ", syscall.args[i], val);
			} else {
				print!("{} = 0x{:x}", syscall.args[i], val);
			}
		}

		println!(")");*/
	}
}

/// This function is called whenever a system call is triggered.
#[no_mangle]
pub extern "C" fn syscall_handler(regs: &mut Regs) {
	// TODO Add switch to disable
	//print_strace(regs, None);

	let id = regs.eax;
	let result = match get_syscall(id) {
		Some(syscall) => (syscall.handler)(regs),

		// The system call doesn't exist. Kill the process with SIGSYS
		None => {
			{
				let mutex = Process::get_current().unwrap();
				let guard = mutex.lock();
				let curr_proc = guard.get_mut();

				// SIGSYS cannot be caught, thus the process will be terminated
				curr_proc.kill(&Signal::SIGSYS, true);
			}

			crate::enter_loop();
		}
	};

	// TODO Add switch to disable
	//print_strace(regs, Some(result));

	regs.set_syscall_return(result);
}
