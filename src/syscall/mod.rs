//! This module handles system calls. A system call is "function" that allows to communcate between
//! userspace and kernelspace.
//!
//! Documentation for each system call can be retrieved from the man. Type the command:
//! `man 2 <syscall>`

mod _exit;
mod _llseek;
mod _newselect;
mod brk;
mod chdir;
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
mod fchdir;
mod fcntl64;
mod fcntl;
mod finit_module;
mod fork;
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
mod getrusage;
mod gettid;
mod getuid32;
mod getuid;
mod init_module;
mod kill;
mod link;
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
mod pipe2;
mod pipe;
mod poll;
mod prlimit64;
mod pselect6;
mod r#break;
mod read;
mod reboot;
mod rt_sigaction;
mod rt_sigprocmask;
mod select;
mod set_thread_area;
mod set_tid_address;
mod setgid;
mod setpgid;
mod setuid;
mod signal;
mod sigreturn;
mod socketpair;
mod statx;
mod time;
mod tkill;
mod truncate;
mod umask;
mod umount;
mod uname;
mod unlink;
mod util;
mod vfork;
mod wait4;
mod wait;
mod waitpid;
mod write;
mod writev;
pub mod ioctl;

use crate::errno::Errno;
use crate::process::Process;
use crate::process::regs::Regs;
use crate::process::signal::Signal;
use crate::process;

//use modify_ldt::modify_ldt;
//use wait::wait;
use _exit::_exit;
use _llseek::_llseek;
use _newselect::_newselect;
use brk::brk;
use chdir::chdir;
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
use fchdir::fchdir;
use fcntl64::fcntl64;
use fcntl::fcntl;
use finit_module::finit_module;
use fork::fork;
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
use getrusage::getrusage;
use gettid::gettid;
use getuid32::getuid32;
use getuid::getuid;
use init_module::init_module;
use ioctl::ioctl;
use kill::kill;
use link::link;
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
use pipe2::pipe2;
use pipe::pipe;
use poll::poll;
use prlimit64::prlimit64;
use pselect6::pselect6;
use r#break::r#break;
use read::read;
use reboot::reboot;
use rt_sigaction::rt_sigaction;
use rt_sigprocmask::rt_sigprocmask;
use select::select;
use set_thread_area::set_thread_area;
use set_tid_address::set_tid_address;
use setgid::setgid;
use setpgid::setpgid;
use setuid::setuid;
use signal::signal;
use sigreturn::sigreturn;
use socketpair::socketpair;
use statx::statx;
use time::time;
use tkill::tkill;
use truncate::truncate;
use umask::umask;
use umount::umount;
use uname::uname;
use unlink::unlink;
use vfork::vfork;
use wait4::wait4;
use waitpid::waitpid;
use write::write;
use writev::writev;

/// Structure representing a system call.
struct Syscall {
	/// The syscall's ID.
	pub id: u32,

	/// The syscall's handler.
	pub handler: &'static dyn Fn(&Regs) -> Result<i32, Errno>,

	/// The syscall's name.
	pub name: &'static str,

	/// The syscall's arguments names.
	pub args: &'static [&'static str],
}

// TODO Fill every arguments
// TODO Allow string arguments to be printed
/// The list of syscall names for each IDs.
const SYSCALLS: &[Syscall] = &[
	// TODO Syscall { id: 0x000, handler: restart_syscall, name: "restart_syscall", args: &[] },
	Syscall { id: 0x001, handler: &_exit, name: "_exit", args: &[] },
	Syscall { id: 0x002, handler: &fork, name: "fork", args: &[] },
	Syscall { id: 0x003, handler: &read, name: "read", args: &[] },
	Syscall { id: 0x004, handler: &write, name: "write", args: &[] },
	Syscall { id: 0x005, handler: &open, name: "open", args: &[] },
	Syscall { id: 0x006, handler: &close, name: "close", args: &[] },
	Syscall { id: 0x007, handler: &waitpid, name: "waitpid", args: &[] },
	Syscall { id: 0x008, handler: &creat, name: "creat", args: &[] },
	Syscall { id: 0x009, handler: &link, name: "link", args: &[] },
	Syscall { id: 0x00a, handler: &unlink, name: "unlink", args: &[] },
	Syscall { id: 0x00b, handler: &execve, name: "execve", args: &[] },
	Syscall { id: 0x00c, handler: &chdir, name: "chdir", args: &[] },
	Syscall { id: 0x00d, handler: &time, name: "time", args: &[] },
	Syscall { id: 0x00e, handler: &mknod, name: "mknod", args: &[] },
	// TODO Syscall { id: 0x00f, handler: chmod, name: "chmod", args: &[] },
	// TODO Syscall { id: 0x010, handler: lchown, name: "lchown", args: &[] },
	Syscall { id: 0x011, handler: &r#break, name: "break", args: &[] },
	// TODO Syscall { id: 0x012, handler: oldstat, name: "oldstat", args: &[] },
	// TODO Syscall { id: 0x013, handler: lseek, name: "lseek", args: &[] },
	Syscall { id: 0x014, handler: &getpid, name: "getpid", args: &[] },
	Syscall { id: 0x015, handler: &mount, name: "mount", args: &[] },
	Syscall { id: 0x016, handler: &umount, name: "umount", args: &[] },
	Syscall { id: 0x017, handler: &setuid, name: "setuid", args: &[] },
	Syscall { id: 0x018, handler: &getuid, name: "getuid", args: &[] },
	// TODO Syscall { id: 0x019, handler: stime, name: "stime", args: &[] },
	// TODO Syscall { id: 0x01a, handler: ptrace, name: "ptrace", args: &[] },
	// TODO Syscall { id: 0x01b, handler: alarm, name: "alarm", args: &[] },
	// TODO Syscall { id: 0x01c, handler: oldfstat, name: "oldfstat", args: &[] },
	// TODO Syscall { id: 0x01d, handler: pause, name: "pause", args: &[] },
	// TODO Syscall { id: 0x01e, handler: utime, name: "utime", args: &[] },
	// TODO Syscall { id: 0x01f, handler: stty, name: "stty", args: &[] },
	// TODO Syscall { id: 0x020, handler: gtty, name: "gtty", args: &[] },
	// TODO Syscall { id: 0x021, handler: access, name: "access", args: &[] },
	// TODO Syscall { id: 0x022, handler: nice, name: "nice", args: &[] },
	// TODO Syscall { id: 0x023, handler: ftime, name: "ftime", args: &[] },
	// TODO Syscall { id: 0x024, handler: sync, name: "sync", args: &[] },
	Syscall { id: 0x025, handler: &kill, name: "kill", args: &[] },
	// TODO Syscall { id: 0x026, handler: rename, name: "rename", args: &[] },
	Syscall { id: 0x027, handler: &mkdir, name: "mkdir", args: &[] },
	// TODO Syscall { id: 0x028, handler: rmdir, name: "rmdir", args: &[] },
	Syscall { id: 0x029, handler: &dup, name: "dup", args: &[] },
	Syscall { id: 0x02a, handler: &pipe, name: "pipe", args: &[] },
	// TODO Syscall { id: 0x02b, handler: times, name: "times", args: &[] },
	// TODO Syscall { id: 0x02c, handler: prof, name: "prof", args: &[] },
	Syscall { id: 0x02d, handler: &brk, name: "brk", args: &[] },
	Syscall { id: 0x02e, handler: &setgid, name: "setgid", args: &[] },
	Syscall { id: 0x02f, handler: &getgid, name: "getgid", args: &[] },
	Syscall { id: 0x030, handler: &signal, name: "signal", args: &[] },
	Syscall { id: 0x031, handler: &geteuid, name: "geteuid", args: &[] },
	Syscall { id: 0x032, handler: &getegid, name: "getegid", args: &[] },
	// TODO Syscall { id: 0x033, handler: acct, name: "acct", args: &[] },
	// TODO Syscall { id: 0x034, handler: umount2, name: "umount2", args: &[] },
	// TODO Syscall { id: 0x035, handler: lock, name: "lock", args: &[] },
	Syscall { id: 0x036, handler: &ioctl, name: "ioctl", args: &[] },
	Syscall { id: 0x037, handler: &fcntl, name: "fcntl", args: &[] },
	// TODO Syscall { id: 0x038, handler: mpx, name: "mpx", args: &[] },
	Syscall { id: 0x039, handler: &setpgid, name: "setpgid", args: &[] },
	// TODO Syscall { id: 0x03a, handler: ulimit, name: "ulimit", args: &[] },
	// TODO Syscall { id: 0x03b, handler: oldolduname, name: "oldolduname", args: &[] },
	Syscall { id: 0x03c, handler: &umask, name: "umask", args: &[] },
	Syscall { id: 0x03d, handler: &chroot, name: "chroot", args: &[] },
	// TODO Syscall { id: 0x03e, handler: ustat, name: "ustat", args: &[] },
	Syscall { id: 0x03f, handler: &dup2, name: "dup2", args: &[] },
	Syscall { id: 0x040, handler: &getppid, name: "getppid", args: &[] },
	// TODO Syscall { id: 0x041, handler: getpgrp, name: "getpgrp", args: &[] },
	// TODO Syscall { id: 0x042, handler: setsid, name: "setsid", args: &[] },
	// TODO Syscall { id: 0x043, handler: sigaction, name: "sigaction", args: &[] },
	// TODO Syscall { id: 0x044, handler: sgetmask, name: "sgetmask", args: &[] },
	// TODO Syscall { id: 0x045, handler: ssetmask, name: "ssetmask", args: &[] },
	// TODO Syscall { id: 0x046, handler: setreuid, name: "setreuid", args: &[] },
	// TODO Syscall { id: 0x047, handler: setregid, name: "setregid", args: &[] },
	// TODO Syscall { id: 0x048, handler: sigsuspend, name: "sigsuspend", args: &[] },
	// TODO Syscall { id: 0x049, handler: sigpending, name: "sigpending", args: &[] },
	// TODO Syscall { id: 0x04a, handler: sethostname, name: "sethostname", args: &[] },
	// TODO Syscall { id: 0x04b, handler: setrlimit, name: "setrlimit", args: &[] },
	// TODO Syscall { id: 0x04c, handler: getrlimit, name: "getrlimit", args: &[] },
	Syscall { id: 0x04d, handler: &getrusage, name: "getrusage", args: &[] },
	// TODO Syscall { id: 0x04e, handler: gettimeofday, name: "gettimeofday", args: &[] },
	// TODO Syscall { id: 0x04f, handler: settimeofday, name: "settimeofday", args: &[] },
	// TODO Syscall { id: 0x050, handler: getgroups, name: "getgroups", args: &[] },
	// TODO Syscall { id: 0x051, handler: setgroups, name: "setgroups", args: &[] },
	Syscall { id: 0x052, handler: &select, name: "select", args: &[] },
	// TODO Syscall { id: 0x053, handler: symlink, name: "symlink", args: &[] },
	// TODO Syscall { id: 0x054, handler: oldlstat, name: "oldlstat", args: &[] },
	// TODO Syscall { id: 0x055, handler: readlink, name: "readlink", args: &[] },
	// TODO Syscall { id: 0x056, handler: uselib, name: "uselib", args: &[] },
	// TODO Syscall { id: 0x057, handler: swapon, name: "swapon", args: &[] },
	Syscall { id: 0x058, handler: &reboot, name: "reboot", args: &[] },
	// TODO Syscall { id: 0x059, handler: readdir, name: "readdir", args: &[] },
	Syscall { id: 0x05a, handler: &mmap, name: "mmap", args: &["addr", "length", "prot", "flags",
		"fd", "offset"] },
	Syscall { id: 0x05b, handler: &munmap, name: "munmap", args: &[] },
	Syscall { id: 0x05c, handler: &truncate, name: "truncate", args: &[] },
	// TODO Syscall { id: 0x05d, handler: ftruncate, name: "ftruncate", args: &[] },
	// TODO Syscall { id: 0x05e, handler: fchmod, name: "fchmod", args: &[] },
	// TODO Syscall { id: 0x05f, handler: fchown, name: "fchown", args: &[] },
	// TODO Syscall { id: 0x060, handler: getpriority, name: "getpriority", args: &[] },
	// TODO Syscall { id: 0x061, handler: setpriority, name: "setpriority", args: &[] },
	// TODO Syscall { id: 0x062, handler: profil, name: "profil", args: &[] },
	// TODO Syscall { id: 0x063, handler: statfs, name: "statfs", args: &[] },
	// TODO Syscall { id: 0x064, handler: fstatfs, name: "fstatfs", args: &[] },
	// TODO Syscall { id: 0x065, handler: ioperm, name: "ioperm", args: &[] },
	// TODO Syscall { id: 0x066, handler: socketcall, name: "socketcall", args: &[] },
	// TODO Syscall { id: 0x067, handler: syslog, name: "syslog", args: &[] },
	// TODO Syscall { id: 0x068, handler: setitimer, name: "setitimer", args: &[] },
	// TODO Syscall { id: 0x069, handler: getitimer, name: "getitimer", args: &[] },
	// TODO Syscall { id: 0x06a, handler: stat, name: "stat", args: &[] },
	// TODO Syscall { id: 0x06b, handler: lstat, name: "lstat", args: &[] },
	// TODO Syscall { id: 0x06c, handler: fstat, name: "fstat", args: &[] },
	// TODO Syscall { id: 0x06d, handler: olduname, name: "olduname", args: &[] },
	// TODO Syscall { id: 0x06e, handler: iopl, name: "iopl", args: &[] },
	// TODO Syscall { id: 0x06f, handler: vhangup, name: "vhangup", args: &[] },
	// TODO Syscall { id: 0x070, handler: idle, name: "idle", args: &[] },
	// TODO Syscall { id: 0x071, handler: vm86old, name: "vm86old", args: &[] },
	Syscall { id: 0x072, handler: &wait4, name: "wait4", args: &[] },
	// TODO Syscall { id: 0x073, handler: swapoff, name: "swapoff", args: &[] },
	// TODO Syscall { id: 0x074, handler: sysinfo, name: "sysinfo", args: &[] },
	// TODO Syscall { id: 0x075, handler: ipc, name: "ipc", args: &[] },
	// TODO Syscall { id: 0x076, handler: fsync, name: "fsync", args: &[] },
	Syscall { id: 0x077, handler: &sigreturn, name: "sigreturn", args: &[] },
	Syscall { id: 0x078, handler: &clone, name: "clone", args: &[] },
	// TODO Syscall { id: 0x079, handler: setdomainname, name: "setdomainname", args: &[] },
	Syscall { id: 0x07a, handler: &uname, name: "uname", args: &[] },
	// TODO Syscall { id: 0x07c, handler: adjtimex, name: "adjtimex", args: &[] },
	// TODO Syscall { id: 0x07d, handler: mprotect, name: "mprotect", args: &[] },
	// TODO Syscall { id: 0x07e, handler: sigprocmask, name: "sigprocmask", args: &[] },
	// TODO Syscall { id: 0x07f, handler: create_module, name: "create_module", args: &[] },
	Syscall { id: 0x080, handler: &init_module, name: "init_module", args: &[] },
	Syscall { id: 0x081, handler: &delete_module, name: "delete_module", args: &[] },
	// TODO Syscall { id: 0x083, handler: quotactl, name: "quotactl", args: &[] },
	Syscall { id: 0x084, handler: &getpgid, name: "getpgid", args: &[] },
	Syscall { id: 0x085, handler: &fchdir, name: "fchdir", args: &[] },
	// TODO Syscall { id: 0x086, handler: bdflush, name: "bdflush", args: &[] },
	// TODO Syscall { id: 0x087, handler: sysfs, name: "sysfs", args: &[] },
	// TODO Syscall { id: 0x088, handler: personality, name: "personality", args: &[] },
	// TODO Syscall { id: 0x089, handler: afs_syscall, name: "afs_syscall", args: &[] },
	// TODO Syscall { id: 0x08a, handler: setfsuid, name: "setfsuid", args: &[] },
	// TODO Syscall { id: 0x08b, handler: setfsgid, name: "setfsgid", args: &[] },
	Syscall { id: 0x08c, handler: &_llseek, name: "_llseek", args: &[] },
	Syscall { id: 0x08d, handler: &getdents, name: "getdents", args: &[] },
	Syscall { id: 0x08e, handler: &_newselect, name: "_newselect", args: &[] },
	// TODO Syscall { id: 0x08f, handler: flock, name: "flock", args: &[] },
	Syscall { id: 0x090, handler: &msync, name: "msync", args: &[] },
	// TODO Syscall { id: 0x091, handler: readv, name: "readv", args: &[] },
	Syscall { id: 0x092, handler: &writev, name: "writev", args: &[] },
	// TODO Syscall { id: 0x093, handler: getsid, name: "getsid", args: &[] },
	// TODO Syscall { id: 0x094, handler: fdatasync, name: "fdatasync", args: &[] },
	// TODO Syscall { id: 0x095, handler: _sysctl, name: "_sysctl", args: &[] },
	// TODO Syscall { id: 0x096, handler: mlock, name: "mlock", args: &[] },
	// TODO Syscall { id: 0x097, handler: munlock, name: "munlock", args: &[] },
	// TODO Syscall { id: 0x098, handler: mlockall, name: "mlockall", args: &[] },
	// TODO Syscall { id: 0x099, handler: munlockall, name: "munlockall", args: &[] },
	// TODO Syscall { id: 0x09a, handler: sched_setparam, name: "sched_setparam", args: &[] },
	// TODO Syscall { id: 0x09b, handler: sched_getparam, name: "sched_getparam", args: &[] },
	// TODO Syscall { id: 0x09c, handler: sched_setscheduler, name: "sched_setscheduler",
	//	args: &[] },
	// TODO Syscall { id: 0x09d, handler: sched_getscheduler, name: "sched_getscheduler",
	//	args: &[] },
	// TODO Syscall { id: 0x09e, handler: sched_yield, name: "sched_yield", args: &[] },
	// TODO Syscall { id: 0x09f, handler: sched_get_priority_max, name: "sched_get_priority_max",
	//	args: &[] },
	// TODO Syscall { id: 0x0a0, handler: sched_get_priority_min, name: "sched_get_priority_min",
	//	args: &[] },
	// TODO Syscall { id: 0x0a1, handler: sched_rr_get_interval, name: "sched_rr_get_interval",
	//	args: &[] },
	Syscall { id: 0x0a2, handler: &nanosleep, name: "nanosleep", args: &[] },
	// TODO Syscall { id: 0x0a3, handler: mremap, name: "mremap", args: &[] },
	// TODO Syscall { id: 0x0a4, handler: setresuid, name: "setresuid", args: &[] },
	// TODO Syscall { id: 0x0a5, handler: getresuid, name: "getresuid", args: &[] },
	// TODO Syscall { id: 0x0a6, handler: vm86, name: "vm86", args: &[] },
	// TODO Syscall { id: 0x0a7, handler: query_module, name: "query_module", args: &[] },
	Syscall { id: 0x0a8, handler: &poll, name: "poll", args: &[] },
	// TODO Syscall { id: 0x0a9, handler: nfsservctl, name: "nfsservctl", args: &[] },
	// TODO Syscall { id: 0x0aa, handler: setresgid, name: "setresgid", args: &[] },
	// TODO Syscall { id: 0x0ab, handler: getresgid, name: "getresgid", args: &[] },
	// TODO Syscall { id: 0x0ac, handler: prctl, name: "prctl", args: &[] },
	// TODO Syscall { id: 0x0ad, handler: rt_sigreturn, name: "rt_sigreturn", args: &[] },
	Syscall { id: 0x0ae, handler: &rt_sigaction, name: "rt_sigaction", args: &[] },
	Syscall { id: 0x0af, handler: &rt_sigprocmask, name: "rt_sigprocmask", args: &[] },
	// TODO Syscall { id: 0x0b0, handler: rt_sigpending, name: "rt_sigpending", args: &[] },
	// TODO Syscall { id: 0x0b1, handler: rt_sigtimedwait, name: "rt_sigtimedwait", args: &[] },
	// TODO Syscall { id: 0x0b2, handler: rt_sigqueueinfo, name: "rt_sigqueueinfo", args: &[] },
	// TODO Syscall { id: 0x0b3, handler: rt_sigsuspend, name: "rt_sigsuspend", args: &[] },
	// TODO Syscall { id: 0x0b4, handler: pread64, name: "pread64", args: &[] },
	// TODO Syscall { id: 0x0b5, handler: pwrite64, name: "pwrite64", args: &[] },
	// TODO Syscall { id: 0x0b6, handler: chown, name: "chown", args: &[] },
	Syscall { id: 0x0b7, handler: &getcwd, name: "getcwd", args: &[] },
	// TODO Syscall { id: 0x0b8, handler: capget, name: "capget", args: &[] },
	// TODO Syscall { id: 0x0b9, handler: capset, name: "capset", args: &[] },
	// TODO Syscall { id: 0x0ba, handler: sigaltstack, name: "sigaltstack", args: &[] },
	// TODO Syscall { id: 0x0bb, handler: sendfile, name: "sendfile", args: &[] },
	// TODO Syscall { id: 0x0bc, handler: getpmsg, name: "getpmsg", args: &[] },
	// TODO Syscall { id: 0x0bd, handler: putpmsg, name: "putpmsg", args: &[] },
	Syscall { id: 0x0be, handler: &vfork, name: "vfork", args: &[] },
	// TODO Syscall { id: 0x0bf, handler: ugetrlimit, name: "ugetrlimit", args: &[] },
	Syscall { id: 0x0c0, handler: &mmap2, name: "mmap2", args: &["addr", "length", "prot", "flags",
		"fd", "offset"] },
	// TODO Syscall { id: 0x0c1, handler: truncate64, name: "truncate64", args: &[] },
	// TODO Syscall { id: 0x0c2, handler: ftruncate64, name: "ftruncate64", args: &[] },
	// TODO Syscall { id: 0x0c3, handler: stat64, name: "stat64", args: &[] },
	// TODO Syscall { id: 0x0c4, handler: lstat64, name: "lstat64", args: &[] },
	// TODO Syscall { id: 0x0c5, handler: fstat64, name: "fstat64", args: &[] },
	// TODO Syscall { id: 0x0c6, handler: lchown32, name: "lchown32", args: &[] },
	Syscall { id: 0x0c7, handler: &getuid32, name: "getuid32", args: &[] },
	Syscall { id: 0x0c8, handler: &getgid32, name: "getgid32", args: &[] },
	Syscall { id: 0x0c9, handler: &geteuid32, name: "geteuid32", args: &[] },
	Syscall { id: 0x0ca, handler: &getegid32, name: "getegid32", args: &[] },
	// TODO Syscall { id: 0x0cb, handler: setreuid32, name: "setreuid32", args: &[] },
	// TODO Syscall { id: 0x0cc, handler: setregid32, name: "setregid32", args: &[] },
	// TODO Syscall { id: 0x0cd, handler: getgroups32, name: "getgroups32", args: &[] },
	// TODO Syscall { id: 0x0ce, handler: setgroups32, name: "setgroups32", args: &[] },
	// TODO Syscall { id: 0x0cf, handler: fchown32, name: "fchown32", args: &[] },
	// TODO Syscall { id: 0x0d0, handler: setresuid32, name: "setresuid32", args: &[] },
	// TODO Syscall { id: 0x0d1, handler: getresuid32, name: "getresuid32", args: &[] },
	// TODO Syscall { id: 0x0d2, handler: setresgid32, name: "setresgid32", args: &[] },
	// TODO Syscall { id: 0x0d3, handler: getresgid32, name: "getresgid32", args: &[] },
	// TODO Syscall { id: 0x0d4, handler: chown32, name: "chown32", args: &[] },
	// TODO Syscall { id: 0x0d5, handler: setuid32, name: "setuid32", args: &[] },
	// TODO Syscall { id: 0x0d6, handler: setgid32, name: "setgid32", args: &[] },
	// TODO Syscall { id: 0x0d7, handler: setfsuid32, name: "setfsuid32", args: &[] },
	// TODO Syscall { id: 0x0d8, handler: setfsgid32, name: "setfsgid32", args: &[] },
	// TODO Syscall { id: 0x0d9, handler: pivot_root, name: "pivot_root", args: &[] },
	// TODO Syscall { id: 0x0da, handler: mincore, name: "mincore", args: &[] },
	Syscall { id: 0x0db, handler: &madvise, name: "madvise", args: &[] },
	Syscall { id: 0x0dc, handler: &getdents64, name: "getdents64", args: &[] },
	Syscall { id: 0x0dd, handler: &fcntl64, name: "fcntl64", args: &[] },
	Syscall { id: 0x0e0, handler: &gettid, name: "gettid", args: &[] },
	// TODO Syscall { id: 0x0e1, handler: readahead, name: "readahead", args: &[] },
	// TODO Syscall { id: 0x0e2, handler: setxattr, name: "setxattr", args: &[] },
	// TODO Syscall { id: 0x0e3, handler: lsetxattr, name: "lsetxattr", args: &[] },
	// TODO Syscall { id: 0x0e4, handler: fsetxattr, name: "fsetxattr", args: &[] },
	// TODO Syscall { id: 0x0e5, handler: getxattr, name: "getxattr", args: &[] },
	// TODO Syscall { id: 0x0e6, handler: lgetxattr, name: "lgetxattr", args: &[] },
	// TODO Syscall { id: 0x0e7, handler: fgetxattr, name: "fgetxattr", args: &[] },
	// TODO Syscall { id: 0x0e8, handler: listxattr, name: "listxattr", args: &[] },
	// TODO Syscall { id: 0x0e9, handler: llistxattr, name: "llistxattr", args: &[] },
	// TODO Syscall { id: 0x0ea, handler: flistxattr, name: "flistxattr", args: &[] },
	// TODO Syscall { id: 0x0eb, handler: removexattr, name: "removexattr", args: &[] },
	// TODO Syscall { id: 0x0ec, handler: lremovexattr, name: "lremovexattr", args: &[] },
	// TODO Syscall { id: 0x0ed, handler: fremovexattr, name: "fremovexattr", args: &[] },
	Syscall { id: 0x0ee, handler: &tkill, name: "tkill", args: &[] },
	// TODO Syscall { id: 0x0ef, handler: sendfile64, name: "sendfile64", args: &[] },
	// TODO Syscall { id: 0x0f0, handler: futex, name: "futex", args: &[] },
	// TODO Syscall { id: 0x0f1, handler: sched_setaffinity, name: "sched_setaffinity",
	//	args: &[] },
	// TODO Syscall { id: 0x0f2, handler: sched_getaffinity, name: "sched_getaffinity",
	//	args: &[] },
	Syscall { id: 0x0f3, handler: &set_thread_area, name: "set_thread_area", args: &[] },
	// TODO Syscall { id: 0x0f4, handler: get_thread_area, name: "get_thread_area", args: &[] },
	// TODO Syscall { id: 0x0f5, handler: io_setup, name: "io_setup", args: &[] },
	// TODO Syscall { id: 0x0f6, handler: io_destroy, name: "io_destroy", args: &[] },
	// TODO Syscall { id: 0x0f7, handler: io_getevents, name: "io_getevents", args: &[] },
	// TODO Syscall { id: 0x0f8, handler: io_submit, name: "io_submit", args: &[] },
	// TODO Syscall { id: 0x0f9, handler: io_cancel, name: "io_cancel", args: &[] },
	// TODO Syscall { id: 0x0fa, handler: fadvise64, name: "fadvise64", args: &[] },
	// TODO Syscall { id: 0x0fc, handler: exit_group, name: "exit_group", args: &[] },
	// TODO Syscall { id: 0x0fd, handler: lookup_dcookie, name: "lookup_dcookie", args: &[] },
	// TODO Syscall { id: 0x0fe, handler: epoll_create, name: "epoll_create", args: &[] },
	// TODO Syscall { id: 0x0ff, handler: epoll_ctl, name: "epoll_ctl", args: &[] },
	// TODO Syscall { id: 0x100, handler: epoll_wait, name: "epoll_wait", args: &[] },
	// TODO Syscall { id: 0x101, handler: remap_file_pages, name: "remap_file_pages", args: &[] },
	Syscall { id: 0x102, handler: &set_tid_address, name: "set_tid_address", args: &[] },
	// TODO Syscall { id: 0x103, handler: timer_create, name: "timer_create", args: &[] },
	// TODO Syscall { id: 0x104, handler: timer_settime, name: "timer_settime", args: &[] },
	// TODO Syscall { id: 0x105, handler: timer_gettime, name: "timer_gettime", args: &[] },
	// TODO Syscall { id: 0x106, handler: timer_getoverrun, name: "timer_getoverrun", args: &[] },
	// TODO Syscall { id: 0x107, handler: timer_delete, name: "timer_delete", args: &[] },
	// TODO Syscall { id: 0x108, handler: clock_settime, name: "clock_settime", args: &[] },
	Syscall { id: 0x109, handler: &clock_gettime, name: "clock_gettime", args: &[] },
	// TODO Syscall { id: 0x10a, handler: clock_getres, name: "clock_getres", args: &[] },
	// TODO Syscall { id: 0x10b, handler: clock_nanosleep, name: "clock_nanosleep", args: &[] },
	// TODO Syscall { id: 0x10c, handler: statfs64, name: "statfs64", args: &[] },
	// TODO Syscall { id: 0x10d, handler: fstatfs64, name: "fstatfs64", args: &[] },
	// TODO Syscall { id: 0x10e, handler: tgkill, name: "tgkill", args: &[] },
	// TODO Syscall { id: 0x10f, handler: utimes, name: "utimes", args: &[] },
	// TODO Syscall { id: 0x110, handler: fadvise64_64, name: "fadvise64_64", args: &[] },
	// TODO Syscall { id: 0x111, handler: vserver, name: "vserver", args: &[] },
	// TODO Syscall { id: 0x112, handler: mbind, name: "mbind", args: &[] },
	// TODO Syscall { id: 0x113, handler: get_mempolicy, name: "get_mempolicy", args: &[] },
	// TODO Syscall { id: 0x114, handler: set_mempolicy, name: "set_mempolicy", args: &[] },
	// TODO Syscall { id: 0x115, handler: mq_open, name: "mq_open", args: &[] },
	// TODO Syscall { id: 0x116, handler: mq_unlink, name: "mq_unlink", args: &[] },
	// TODO Syscall { id: 0x117, handler: mq_timedsend, name: "mq_timedsend", args: &[] },
	// TODO Syscall { id: 0x118, handler: mq_timedreceive, name: "mq_timedreceive", args: &[] },
	// TODO Syscall { id: 0x119, handler: mq_notify, name: "mq_notify", args: &[] },
	// TODO Syscall { id: 0x11a, handler: mq_getsetattr, name: "mq_getsetattr", args: &[] },
	// TODO Syscall { id: 0x11b, handler: kexec_load, name: "kexec_load", args: &[] },
	// TODO Syscall { id: 0x11c, handler: waitid, name: "waitid", args: &[] },
	// TODO Syscall { id: 0x11e, handler: add_key, name: "add_key", args: &[] },
	// TODO Syscall { id: 0x11f, handler: request_key, name: "request_key", args: &[] },
	// TODO Syscall { id: 0x120, handler: keyctl, name: "keyctl", args: &[] },
	// TODO Syscall { id: 0x121, handler: ioprio_set, name: "ioprio_set", args: &[] },
	// TODO Syscall { id: 0x122, handler: ioprio_get, name: "ioprio_get", args: &[] },
	// TODO Syscall { id: 0x123, handler: inotify_init, name: "inotify_init", args: &[] },
	// TODO Syscall { id: 0x124, handler: inotify_add_watch, name: "inotify_add_watch",
	//	args: &[] },
	// TODO Syscall { id: 0x125, handler: inotify_rm_watch, name: "inotify_rm_watch", args: &[] },
	// TODO Syscall { id: 0x126, handler: migrate_pages, name: "migrate_pages", args: &[] },
	// TODO Syscall { id: 0x127, handler: openat, name: "openat", args: &[] },
	// TODO Syscall { id: 0x128, handler: mkdirat, name: "mkdirat", args: &[] },
	// TODO Syscall { id: 0x129, handler: mknodat, name: "mknodat", args: &[] },
	// TODO Syscall { id: 0x12a, handler: fchownat, name: "fchownat", args: &[] },
	// TODO Syscall { id: 0x12b, handler: futimesat, name: "futimesat", args: &[] },
	// TODO Syscall { id: 0x12c, handler: fstatat64, name: "fstatat64", args: &[] },
	// TODO Syscall { id: 0x12d, handler: unlinkat, name: "unlinkat", args: &[] },
	// TODO Syscall { id: 0x12e, handler: renameat, name: "renameat", args: &[] },
	// TODO Syscall { id: 0x12f, handler: linkat, name: "linkat", args: &[] },
	// TODO Syscall { id: 0x130, handler: symlinkat, name: "symlinkat", args: &[] },
	// TODO Syscall { id: 0x131, handler: readlinkat, name: "readlinkat", args: &[] },
	// TODO Syscall { id: 0x132, handler: fchmodat, name: "fchmodat", args: &[] },
	// TODO Syscall { id: 0x133, handler: faccessat, name: "faccessat", args: &[] },
	Syscall { id: 0x134, handler: &pselect6, name: "pselect6", args: &[] },
	// TODO Syscall { id: 0x135, handler: ppoll, name: "ppoll", args: &[] },
	// TODO Syscall { id: 0x136, handler: unshare, name: "unshare", args: &[] },
	// TODO Syscall { id: 0x137, handler: set_robust_list, name: "set_robust_list", args: &[] },
	// TODO Syscall { id: 0x138, handler: get_robust_list, name: "get_robust_list", args: &[] },
	// TODO Syscall { id: 0x139, handler: splice, name: "splice", args: &[] },
	// TODO Syscall { id: 0x13a, handler: sync_file_range, name: "sync_file_range", args: &[] },
	// TODO Syscall { id: 0x13b, handler: tee, name: "tee", args: &[] },
	// TODO Syscall { id: 0x13c, handler: vmsplice, name: "vmsplice", args: &[] },
	// TODO Syscall { id: 0x13d, handler: move_pages, name: "move_pages", args: &[] },
	// TODO Syscall { id: 0x13e, handler: getcpu, name: "getcpu", args: &[] },
	// TODO Syscall { id: 0x13f, handler: epoll_pwait, name: "epoll_pwait", args: &[] },
	// TODO Syscall { id: 0x140, handler: utimensat, name: "utimensat", args: &[] },
	// TODO Syscall { id: 0x141, handler: signalfd, name: "signalfd", args: &[] },
	// TODO Syscall { id: 0x142, handler: timerfd_create, name: "timerfd_create", args: &[] },
	// TODO Syscall { id: 0x143, handler: eventfd, name: "eventfd", args: &[] },
	// TODO Syscall { id: 0x144, handler: fallocate, name: "fallocate", args: &[] },
	// TODO Syscall { id: 0x145, handler: timerfd_settime, name: "timerfd_settime", args: &[] },
	// TODO Syscall { id: 0x146, handler: timerfd_gettime, name: "timerfd_gettime", args: &[] },
	// TODO Syscall { id: 0x147, handler: signalfd4, name: "signalfd4", args: &[] },
	// TODO Syscall { id: 0x148, handler: eventfd2, name: "eventfd2", args: &[] },
	// TODO Syscall { id: 0x149, handler: epoll_create1, name: "epoll_create1", args: &[] },
	// TODO Syscall { id: 0x14a, handler: dup3, name: "dup3", args: &[] },
	Syscall { id: 0x14b, handler: &pipe2, name: "pipe2", args: &[] },
	// TODO Syscall { id: 0x14c, handler: inotify_init1, name: "inotify_init1", args: &[] },
	// TODO Syscall { id: 0x14d, handler: preadv, name: "preadv", args: &[] },
	// TODO Syscall { id: 0x14e, handler: pwritev, name: "pwritev", args: &[] },
	// TODO Syscall { id: 0x14f, handler: rt_tgsigqueueinfo, name: "rt_tgsigqueueinfo",
	//	args: &[] },
	// TODO Syscall { id: 0x150, handler: perf_event_open, name: "perf_event_open", args: &[] },
	// TODO Syscall { id: 0x151, handler: recvmmsg, name: "recvmmsg", args: &[] },
	// TODO Syscall { id: 0x152, handler: fanotify_init, name: "fanotify_init", args: &[] },
	// TODO Syscall { id: 0x153, handler: fanotify_mark, name: "fanotify_mark", args: &[] },
	Syscall { id: 0x154, handler: &prlimit64, name: "prlimit64", args: &[] },
	// TODO Syscall { id: 0x155, handler: name_to_handle_at, name: "name_to_handle_at",
	//	args: &[] },
	// TODO Syscall { id: 0x156, handler: open_by_handle_at, name: "open_by_handle_at",
	//	args: &[] },
	// TODO Syscall { id: 0x157, handler: clock_adjtime, name: "clock_adjtime", args: &[] },
	// TODO Syscall { id: 0x158, handler: syncfs, name: "syncfs", args: &[] },
	// TODO Syscall { id: 0x159, handler: sendmmsg, name: "sendmmsg", args: &[] },
	// TODO Syscall { id: 0x15a, handler: setns, name: "setns", args: &[] },
	// TODO Syscall { id: 0x15b, handler: process_vm_readv, name: "process_vm_readv", args: &[] },
	// TODO Syscall { id: 0x15c, handler: process_vm_writev, name: "process_vm_writev",
	//	args: &[] },
	// TODO Syscall { id: 0x15d, handler: kcmp, name: "kcmp", args: &[] },
	Syscall { id: 0x15e, handler: &finit_module, name: "finit_module", args: &[] },
	// TODO Syscall { id: 0x15f, handler: sched_setattr, name: "sched_setattr", args: &[] },
	// TODO Syscall { id: 0x160, handler: sched_getattr, name: "sched_getattr", args: &[] },
	// TODO Syscall { id: 0x161, handler: renameat2, name: "renameat2", args: &[] },
	// TODO Syscall { id: 0x162, handler: seccomp, name: "seccomp", args: &[] },
	// TODO Syscall { id: 0x163, handler: getrandom, name: "getrandom", args: &[] },
	// TODO Syscall { id: 0x164, handler: memfd_create, name: "memfd_create", args: &[] },
	// TODO Syscall { id: 0x165, handler: bpf, name: "bpf", args: &[] },
	// TODO Syscall { id: 0x166, handler: execveat, name: "execveat", args: &[] },
	// TODO Syscall { id: 0x167, handler: socket, name: "socket", args: &[] },
	Syscall { id: 0x168, handler: &socketpair, name: "socketpair", args: &[] },
	// TODO Syscall { id: 0x169, handler: bind, name: "bind", args: &[] },
	// TODO Syscall { id: 0x16a, handler: connect, name: "connect", args: &[] },
	// TODO Syscall { id: 0x16b, handler: listen, name: "listen", args: &[] },
	// TODO Syscall { id: 0x16c, handler: accept4, name: "accept4", args: &[] },
	// TODO Syscall { id: 0x16d, handler: getsockopt, name: "getsockopt", args: &[] },
	// TODO Syscall { id: 0x16e, handler: setsockopt, name: "setsockopt", args: &[] },
	// TODO Syscall { id: 0x16f, handler: getsockname, name: "getsockname", args: &[] },
	// TODO Syscall { id: 0x170, handler: getpeername, name: "getpeername", args: &[] },
	// TODO Syscall { id: 0x171, handler: sendto, name: "sendto", args: &[] },
	// TODO Syscall { id: 0x172, handler: sendmsg, name: "sendmsg", args: &[] },
	// TODO Syscall { id: 0x173, handler: recvfrom, name: "recvfrom", args: &[] },
	// TODO Syscall { id: 0x174, handler: recvmsg, name: "recvmsg", args: &[] },
	// TODO Syscall { id: 0x175, handler: shutdown, name: "shutdown", args: &[] },
	// TODO Syscall { id: 0x176, handler: userfaultfd, name: "userfaultfd", args: &[] },
	// TODO Syscall { id: 0x177, handler: membarrier, name: "membarrier", args: &[] },
	// TODO Syscall { id: 0x178, handler: mlock2, name: "mlock2", args: &[] },
	// TODO Syscall { id: 0x179, handler: copy_file_range, name: "copy_file_range", args: &[] },
	// TODO Syscall { id: 0x17a, handler: preadv2, name: "preadv2", args: &[] },
	// TODO Syscall { id: 0x17b, handler: pwritev2, name: "pwritev2", args: &[] },
	// TODO Syscall { id: 0x17c, handler: pkey_mprotect, name: "pkey_mprotect", args: &[] },
	// TODO Syscall { id: 0x17d, handler: pkey_alloc, name: "pkey_alloc", args: &[] },
	// TODO Syscall { id: 0x17e, handler: pkey_free, name: "pkey_free", args: &[] },
	Syscall { id: 0x17f, handler: &statx, name: "statx", args: &[] },
	// TODO Syscall { id: 0x180, handler: arch_prctl, name: "arch_prctl", args: &[] },
	// TODO Syscall { id: 0x181, handler: io_pgetevents, name: "io_pgetevents", args: &[] },
	// TODO Syscall { id: 0x182, handler: rseq, name: "rseq", args: &[] },
	// TODO Syscall { id: 0x189, handler: semget, name: "semget", args: &[] },
	// TODO Syscall { id: 0x18a, handler: semctl, name: "semctl", args: &[] },
	// TODO Syscall { id: 0x18b, handler: shmget, name: "shmget", args: &[] },
	// TODO Syscall { id: 0x18c, handler: shmctl, name: "shmctl", args: &[] },
	// TODO Syscall { id: 0x18d, handler: shmat, name: "shmat", args: &[] },
	// TODO Syscall { id: 0x18e, handler: shmdt, name: "shmdt", args: &[] },
	// TODO Syscall { id: 0x18f, handler: msgget, name: "msgget", args: &[] },
	// TODO Syscall { id: 0x190, handler: msgsnd, name: "msgsnd", args: &[] },
	// TODO Syscall { id: 0x191, handler: msgrcv, name: "msgrcv", args: &[] },
	// TODO Syscall { id: 0x192, handler: msgctl, name: "msgctl", args: &[] },
	Syscall { id: 0x193, handler: &clock_gettime64, name: "clock_gettime64", args: &[] },
	// TODO Syscall { id: 0x194, handler: clock_settime64, name: "clock_settime64", args: &[] },
	// TODO Syscall { id: 0x195, handler: clock_adjtime64, name: "clock_adjtime64", args: &[] },
	// TODO Syscall { id: 0x196, handler: clock_getres_time64, name: "clock_getres_time64",
	//	args: &[] },
	// TODO Syscall { id: 0x197, handler: clock_nanosleep_time64, name: "clock_nanosleep_time64",
	//	args: &[] },
	// TODO Syscall { id: 0x198, handler: timer_gettime64, name: "timer_gettime64", args: &[] },
	// TODO Syscall { id: 0x199, handler: timer_settime64, name: "timer_settime64", args: &[] },
	// TODO Syscall { id: 0x19a, handler: timerfd_gettime64, name: "timerfd_gettime64",
	//	args: &[] },
	// TODO Syscall { id: 0x19b, handler: timerfd_settime64, name: "timerfd_settime64",
	//	args: &[] },
	// TODO Syscall { id: 0x19c, handler: utimensat_time64, name: "utimensat_time64", args: &[] },
	// TODO Syscall { id: 0x19d, handler: pselect6_time64, name: "pselect6_time64", args: &[] },
	// TODO Syscall { id: 0x19e, handler: ppoll_time64, name: "ppoll_time64", args: &[] },
	// TODO Syscall { id: 0x1a0, handler: io_pgetevents_time64, name: "io_pgetevents_time64",
	//	args: &[] },
	// TODO Syscall { id: 0x1a1, handler: recvmmsg_time64, name: "recvmmsg_time64", args: &[] },
	// TODO Syscall { id: 0x1a2, handler: mq_timedsend_time64, name: "mq_timedsend_time64",
	//	args: &[] },
	// TODO Syscall { id: 0x1a3, handler: mq_timedreceive_time64, name: "mq_timedreceive_time64",
	//	args: &[] },
	// TODO Syscall { id: 0x1a4, handler: semtimedop_time64, name: "semtimedop_time64",
	//	args: &[] },
	// TODO Syscall { id: 0x1a5, handler: rt_sigtimedwait_time64, name: "rt_sigtimedwait_time64",
	//	args: &[] },
	// TODO Syscall { id: 0x1a6, handler: futex_time64, name: "futex_time64", args: &[] },
	// TODO Syscall { id: 0x1a7, handler: sched_rr_get_interval_time64,
	//	name: "sched_rr_get_interval_time64", args: &[] },
	// TODO Syscall { id: 0x1a8, handler: pidfd_send_signal, name: "pidfd_send_signal",
	//	args: &[] },
	// TODO Syscall { id: 0x1a9, handler: io_uring_setup, name: "io_uring_setup", args: &[] },
	// TODO Syscall { id: 0x1aa, handler: io_uring_enter, name: "io_uring_enter", args: &[] },
	// TODO Syscall { id: 0x1ab, handler: io_uring_register, name: "io_uring_register",
	//	args: &[] },
	// TODO Syscall { id: 0x1ac, handler: open_tree, name: "open_tree", args: &[] },
	// TODO Syscall { id: 0x1ad, handler: move_mount, name: "move_mount", args: &[] },
	// TODO Syscall { id: 0x1ae, handler: fsopen, name: "fsopen", args: &[] },
	// TODO Syscall { id: 0x1af, handler: fsconfig, name: "fsconfig", args: &[] },
	// TODO Syscall { id: 0x1b0, handler: fsmount, name: "fsmount", args: &[] },
	// TODO Syscall { id: 0x1b1, handler: fspick, name: "fspick", args: &[] },
	// TODO Syscall { id: 0x1b2, handler: pidfd_open, name: "pidfd_open", args: &[] },
	// TODO Syscall { id: 0x1b3, handler: clone3, name: "clone3", args: &[] },
	// TODO Syscall { id: 0x1b4, handler: close_range, name: "close_range", args: &[] },
	// TODO Syscall { id: 0x1b5, handler: openat2, name: "openat2", args: &[] },
	// TODO Syscall { id: 0x1b6, handler: pidfd_getfd, name: "pidfd_getfd", args: &[] },
	// TODO Syscall { id: 0x1b7, handler: faccessat2, name: "faccessat2", args: &[] },
	// TODO Syscall { id: 0x1b8, handler: process_madvise, name: "process_madvise", args: &[] },
	// TODO Syscall { id: 0x1b9, handler: epoll_pwait2, name: "epoll_pwait2", args: &[] },
	// TODO Syscall { id: 0x1ba, handler: mount_setattr, name: "mount_setattr", args: &[] },
	// TODO Syscall { id: 0x1bb, handler: quotactl_fd, name: "quotactl_fd", args: &[] },
	// TODO Syscall { id: 0x1bc, handler: landlock_create_ruleset, name: "landlock_create_ruleset",
	//	args: &[] },
	// TODO Syscall { id: 0x1bd, handler: landlock_add_rule, name: "landlock_add_rule",
	//	args: &[] },
	// TODO Syscall { id: 0x1be, handler: landlock_restrict_self, name: "landlock_restrict_self",
	//	args: &[] },
	// TODO Syscall { id: 0x1bf, handler: memfd_secret, name: "memfd_secret", args: &[] },
	// TODO Syscall { id: 0x1c0, handler: process_mrelease, name: "process_mrelease", args: &[] },
	// TODO Syscall { id: 0x1c1, handler: futex_waitv, name: "futex_waitv", args: &[] },
	// TODO Syscall { id: 0x1c2, handler: set_mempolicy_home_node, name: "set_mempolicy_home_node",
	//	args: &[] },
];

/// Prints the trace for a syscall.
/// `regs` are the registers passed to the syscall.
/// `result` is the result of the syscall.
fn print_strace(regs: &Regs, result: Option<Result<i32, Errno>>) {
	let pid = {
		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock();
		let proc = guard.get_mut();
		proc.get_pid()
	};

	// Getting syscall name
	let id = regs.eax;

	// TODO Optimize (holes in the syscall table)
	let syscall = match &SYSCALLS.binary_search_by(| s | s.id.cmp(&id)) {
		Ok(syscall) => &SYSCALLS[*syscall],
		Err(_) => {
			println!("invalid syscall (pid: {}): {:x}", pid, id);
			return;
		},
	};

	if let Some(result) = result {
		match result {
			Ok(val) => println!(" -> Ok(0x{:x})", val as usize),
			Err(errno) => println!(" -> Errno({})", errno),
		}
	} else {
		print!("strace start (pid: {}): {}(", pid, syscall.name);

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

// TODO Add a strace-like feature gated by a compilation option
/// This function is called whenever a system call is triggered.
#[no_mangle]
pub extern "C" fn syscall_handler(regs: &mut Regs) {
	let id = regs.eax;

	// TODO Add switch to disable
	//print_strace(regs, None);

	// TODO Optimize (holes in the syscall table)
	let result = match &SYSCALLS.binary_search_by(| s | s.id.cmp(&id)) {
		Ok(syscall) => (SYSCALLS[*syscall].handler)(regs),

		// The system call doesn't exist. Killing the process with SIGSYS
		Err(_) => {
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

	// TODO Add switch to disable
	//print_strace(regs, Some(result));

	// Setting the return value
	let retval = {
		if let Ok(val) = result {
			val as _
		} else {
			(-result.unwrap_err().as_int()) as _
		}
	};
	regs.eax = retval;
}
