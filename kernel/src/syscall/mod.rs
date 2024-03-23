/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! This module handles system calls. A system call is "function" that allows to
//! communicate between userspace and kernelspace.
//!
//! Documentation for each system call can be retrieved from the man. Type the
//! command: `man 2 <syscall>`

mod _exit;
mod _llseek;
mod _newselect;
mod access;
mod arch_prctl;
mod bind;
mod r#break;
mod brk;
mod chdir;
mod chmod;
mod chown;
mod chown32;
mod chroot;
mod clock_gettime;
mod clock_gettime64;
mod clone;
mod close;
mod connect;
mod creat;
mod delete_module;
mod dup;
mod dup2;
mod execve;
mod exit_group;
mod faccessat;
mod faccessat2;
mod fadvise64_64;
mod fchdir;
mod fchmod;
mod fchmodat;
mod fcntl;
mod fcntl64;
mod finit_module;
mod fork;
mod fstat64;
mod fstatfs;
mod fstatfs64;
mod fsync;
mod getcwd;
mod getdents;
mod getdents64;
mod getegid;
mod getegid32;
mod geteuid;
mod geteuid32;
mod getgid;
mod getgid32;
mod getpgid;
mod getpid;
mod getppid;
mod getrandom;
mod getrusage;
mod getsockname;
mod getsockopt;
mod gettid;
mod getuid;
mod getuid32;
mod init_module;
pub mod ioctl;
mod kill;
mod lchown;
mod link;
mod linkat;
mod madvise;
mod mkdir;
mod mknod;
mod mmap;
mod mmap2;
mod mount;
mod mprotect;
mod msync;
mod munmap;
mod nanosleep;
mod open;
mod openat;
mod pipe;
mod pipe2;
mod poll;
mod preadv;
mod preadv2;
mod prlimit64;
mod pselect6;
mod pwritev;
mod pwritev2;
mod read;
mod readlink;
mod readv;
mod reboot;
mod rename;
mod renameat2;
mod rmdir;
mod rt_sigaction;
mod rt_sigprocmask;
mod sched_yield;
mod select;
mod sendto;
mod set_thread_area;
mod set_tid_address;
mod setgid;
mod setgid32;
mod sethostname;
mod setpgid;
mod setsockopt;
mod setuid;
mod setuid32;
mod shutdown;
mod signal;
mod sigreturn;
mod socket;
mod socketpair;
mod splice;
mod statfs;
mod statfs64;
mod statx;
mod symlink;
mod symlinkat;
mod syncfs;
mod time;
mod timer_create;
mod timer_delete;
mod timer_settime;
mod tkill;
mod truncate;
mod umask;
mod umount;
mod uname;
mod unlink;
mod unlinkat;
mod util;
mod utimensat;
mod vfork;
mod wait;
mod wait4;
mod waitpid;
mod write;
mod writev;

//use wait::wait;
use crate::process::{regs::Regs, signal::Signal, Process};
use _exit::_exit;
use _llseek::_llseek;
use _newselect::_newselect;
use access::access;
use arch_prctl::arch_prctl;
use bind::bind;
use brk::brk;
use chdir::chdir;
use chmod::chmod;
use chown::chown;
use chown32::chown32;
use chroot::chroot;
use clock_gettime::clock_gettime;
use clock_gettime64::clock_gettime64;
use clone::clone;
use close::close;
use connect::connect;
use creat::creat;
use delete_module::delete_module;
use dup::dup;
use dup2::dup2;
use execve::execve;
use exit_group::exit_group;
use faccessat::faccessat;
use faccessat2::faccessat2;
use fadvise64_64::fadvise64_64;
use fchdir::fchdir;
use fchmod::fchmod;
use fchmodat::fchmodat;
use fcntl::fcntl;
use fcntl64::fcntl64;
use finit_module::finit_module;
use fork::fork;
use fstat64::fstat64;
use fstatfs::fstatfs;
use fstatfs64::fstatfs64;
use fsync::fsync;
use getcwd::getcwd;
use getdents::getdents;
use getdents64::getdents64;
use getegid::getegid;
use getegid32::getegid32;
use geteuid::geteuid;
use geteuid32::geteuid32;
use getgid::getgid;
use getgid32::getgid32;
use getpgid::getpgid;
use getpid::getpid;
use getppid::getppid;
use getrandom::getrandom;
use getrusage::getrusage;
use getsockname::getsockname;
use getsockopt::getsockopt;
use gettid::gettid;
use getuid::getuid;
use getuid32::getuid32;
use init_module::init_module;
use ioctl::ioctl;
use kill::kill;
use lchown::lchown;
use link::link;
use linkat::linkat;
use madvise::madvise;
use mkdir::mkdir;
use mknod::mknod;
use mmap::mmap;
use mmap2::mmap2;
use mount::mount;
use mprotect::mprotect;
use msync::msync;
use munmap::munmap;
use nanosleep::nanosleep;
use open::open;
use openat::openat;
use pipe::pipe;
use pipe2::pipe2;
use poll::poll;
use preadv::preadv;
use preadv2::preadv2;
use prlimit64::prlimit64;
use pselect6::pselect6;
use pwritev::pwritev;
use pwritev2::pwritev2;
use r#break::r#break;
use read::read;
use readlink::readlink;
use readv::readv;
use reboot::reboot;
use rename::rename;
use renameat2::renameat2;
use rmdir::rmdir;
use rt_sigaction::rt_sigaction;
use rt_sigprocmask::rt_sigprocmask;
use sched_yield::sched_yield;
use select::select;
use sendto::sendto;
use set_thread_area::set_thread_area;
use set_tid_address::set_tid_address;
use setgid::setgid;
use setgid32::setgid32;
use sethostname::sethostname;
use setpgid::setpgid;
use setsockopt::setsockopt;
use setuid::setuid;
use setuid32::setuid32;
use shutdown::shutdown;
use signal::signal;
use sigreturn::sigreturn;
use socket::socket;
use socketpair::socketpair;
use splice::splice;
use statfs::statfs;
use statfs64::statfs64;
use statx::statx;
use symlink::symlink;
use symlinkat::symlinkat;
use syncfs::syncfs;
use time::time;
use timer_create::timer_create;
use timer_delete::timer_delete;
use timer_settime::timer_settime;
use tkill::tkill;
use truncate::truncate;
use umask::umask;
use umount::umount;
use uname::uname;
use unlink::unlink;
use unlinkat::unlinkat;
use utils::errno::EResult;
use utimensat::utimensat;
use vfork::vfork;
use wait4::wait4;
use waitpid::waitpid;
use write::write;
use writev::writev;

/// The ID of the `sigreturn` system call, for use by the signal trampoline.
pub const SIGRETURN_ID: u32 = 0x077;

type SyscallHandler = &'static dyn Fn(&Regs) -> EResult<i32>;

/// Returns the system call associated with the given ID `id`.
///
/// If the syscall doesn't exist, the function returns `None`.
fn get_syscall(id: u32) -> Option<SyscallHandler> {
	match id {
		0x001 => Some(&_exit),
		0x002 => Some(&fork),
		0x003 => Some(&read),
		0x004 => Some(&write),
		0x005 => Some(&open),
		0x006 => Some(&close),
		0x007 => Some(&waitpid),
		0x008 => Some(&creat),
		0x009 => Some(&link),
		0x00a => Some(&unlink),
		0x00b => Some(&execve),
		0x00c => Some(&chdir),
		0x00d => Some(&time),
		0x00e => Some(&mknod),
		0x00f => Some(&chmod),
		0x010 => Some(&lchown),
		0x011 => Some(&r#break),
		// TODO 0x012 => Some(&oldstat),
		// TODO 0x013 => Some(&lseek),
		0x014 => Some(&getpid),
		0x015 => Some(&mount),
		0x016 => Some(&umount),
		0x017 => Some(&setuid),
		0x018 => Some(&getuid),
		// TODO 0x019 => Some(&stime),
		// TODO 0x01a => Some(&ptrace),
		// TODO 0x01b => Some(&alarm),
		// TODO 0x01c => Some(&oldfstat),
		// TODO 0x01d => Some(&pause),
		// TODO 0x01e => Some(&utime),
		// TODO 0x01f => Some(&stty),
		// TODO 0x020 => Some(&gtty),
		0x021 => Some(&access),
		// TODO 0x022 => Some(&nice),
		// TODO 0x023 => Some(&ftime),
		// TODO 0x024 => Some(&sync),
		0x025 => Some(&kill),
		0x026 => Some(&rename),
		0x027 => Some(&mkdir),
		0x028 => Some(&rmdir),
		0x029 => Some(&dup),
		0x02a => Some(&pipe),
		// TODO 0x02b => Some(&times),
		// TODO 0x02c => Some(&prof),
		0x02d => Some(&brk),
		0x02e => Some(&setgid),
		0x02f => Some(&getgid),
		0x030 => Some(&signal),
		0x031 => Some(&geteuid),
		0x032 => Some(&getegid),
		// TODO 0x033 => Some(&acct),
		// TODO 0x034 => Some(&umount2),
		// TODO 0x035 => Some(&lock),
		0x036 => Some(&ioctl),
		0x037 => Some(&fcntl),
		// TODO 0x038 => Some(&mpx),
		0x039 => Some(&setpgid),
		// TODO 0x03a => Some(&ulimit),
		// TODO 0x03b => Some(&oldolduname),
		0x03c => Some(&umask),
		0x03d => Some(&chroot),
		// TODO 0x03e => Some(&ustat),
		0x03f => Some(&dup2),
		0x040 => Some(&getppid),
		// TODO 0x041 => Some(&getpgrp),
		// TODO 0x042 => Some(&setsid),
		// TODO 0x043 => Some(&sigaction),
		// TODO 0x044 => Some(&sgetmask),
		// TODO 0x045 => Some(&ssetmask),
		// TODO 0x046 => Some(&setreuid),
		// TODO 0x047 => Some(&setregid),
		// TODO 0x048 => Some(&sigsuspend),
		// TODO 0x049 => Some(&sigpending),
		0x04a => Some(&sethostname),
		// TODO 0x04b => Some(&setrlimit),
		// TODO 0x04c => Some(&getrlimit),
		0x04d => Some(&getrusage),
		// TODO 0x04e => Some(&gettimeofday),
		// TODO 0x04f => Some(&settimeofday),
		// TODO 0x050 => Some(&getgroups),
		// TODO 0x051 => Some(&setgroups),
		0x052 => Some(&select),
		0x053 => Some(&symlink),
		// TODO 0x054 => Some(&oldlstat),
		0x055 => Some(&readlink),
		// TODO 0x056 => Some(&uselib),
		// TODO 0x057 => Some(&swapon),
		0x058 => Some(&reboot),
		// TODO 0x059 => Some(&readdir),
		0x05a => Some(&mmap),
		0x05b => Some(&munmap),
		0x05c => Some(&truncate),
		// TODO 0x05d => Some(&ftruncate),
		0x05e => Some(&fchmod),
		// TODO 0x05f => Some(&fchown),
		// TODO 0x060 => Some(&getpriority),
		// TODO 0x061 => Some(&setpriority),
		// TODO 0x062 => Some(&profil),
		0x063 => Some(&statfs),
		0x064 => Some(&fstatfs),
		// TODO 0x065 => Some(&ioperm),
		// TODO 0x066 => Some(&socketcall),
		// TODO 0x067 => Some(&syslog),
		// TODO 0x068 => Some(&setitimer),
		// TODO 0x069 => Some(&getitimer),
		// TODO 0x06a => Some(&stat),
		// TODO 0x06b => Some(&lstat),
		// TODO 0x06c => Some(&fstat),
		// TODO 0x06d => Some(&olduname),
		// TODO 0x06e => Some(&iopl),
		// TODO 0x06f => Some(&vhangup),
		// TODO 0x070 => Some(&idle),
		// TODO 0x071 => Some(&vm86old),
		0x072 => Some(&wait4),
		// TODO 0x073 => Some(&swapoff),
		// TODO 0x074 => Some(&sysinfo),
		// TODO 0x075 => Some(&ipc),
		0x076 => Some(&fsync),
		SIGRETURN_ID => Some(&sigreturn),
		0x078 => Some(&clone),
		// TODO 0x079 => Some(&setdomainname),
		0x07a => Some(&uname),
		// TODO 0x07c => Some(&adjtimex),
		0x07d => Some(&mprotect),
		// TODO 0x07e => Some(&sigprocmask),
		// TODO 0x07f => Some(&create_module),
		0x080 => Some(&init_module),
		0x081 => Some(&delete_module),
		// TODO 0x083 => Some(&quotactl),
		0x084 => Some(&getpgid),
		0x085 => Some(&fchdir),
		// TODO 0x086 => Some(&bdflush),
		// TODO 0x087 => Some(&sysfs),
		// TODO 0x088 => Some(&personality),
		// TODO 0x089 => Some(&afs_syscall),
		// TODO 0x08a => Some(&setfsuid),
		// TODO 0x08b => Some(&setfsgid),
		0x08c => Some(&_llseek),
		0x08d => Some(&getdents),
		0x08e => Some(&_newselect),
		// TODO 0x08f => Some(&flock),
		0x090 => Some(&msync),
		0x091 => Some(&readv),
		0x092 => Some(&writev),
		// TODO 0x093 => Some(&getsid),
		// TODO 0x094 => Some(&fdatasync),
		// TODO 0x095 => Some(&_sysctl),
		// TODO 0x096 => Some(&mlock),
		// TODO 0x097 => Some(&munlock),
		// TODO 0x098 => Some(&mlockall),
		// TODO 0x099 => Some(&munlockall),
		// TODO 0x09a => Some(&sched_setparam),
		// TODO 0x09b => Some(&sched_getparam),
		// TODO 0x09c => Some(&sched_setscheduler),
		// TODO 0x09d => Some(&sched_getscheduler),
		0x09e => Some(&sched_yield),
		// TODO 0x09f => Some(&sched_get_priority_max),
		// TODO 0x0a0 => Some(&sched_get_priority_min),
		// TODO 0x0a1 => Some(&sched_rr_get_interval),
		0x0a2 => Some(&nanosleep),
		// TODO 0x0a3 => Some(&mremap),
		// TODO 0x0a4 => Some(&setresuid),
		// TODO 0x0a5 => Some(&getresuid),
		// TODO 0x0a6 => Some(&vm86),
		// TODO 0x0a7 => Some(&query_module),
		0x0a8 => Some(&poll),
		// TODO 0x0a9 => Some(&nfsservctl),
		// TODO 0x0aa => Some(&setresgid),
		// TODO 0x0ab => Some(&getresgid),
		// TODO 0x0ac => Some(&prctl),
		// TODO 0x0ad => Some(&rt_sigreturn),
		0x0ae => Some(&rt_sigaction),
		0x0af => Some(&rt_sigprocmask),
		// TODO 0x0b0 => Some(&rt_sigpending),
		// TODO 0x0b1 => Some(&rt_sigtimedwait),
		// TODO 0x0b2 => Some(&rt_sigqueueinfo),
		// TODO 0x0b3 => Some(&rt_sigsuspend),
		// TODO 0x0b4 => Some(&pread64),
		// TODO 0x0b5 => Some(&pwrite64),
		0x0b6 => Some(&chown),
		0x0b7 => Some(&getcwd),
		// TODO 0x0b8 => Some(&capget),
		// TODO 0x0b9 => Some(&capset),
		// TODO 0x0ba => Some(&sigaltstack),
		// TODO 0x0bb => Some(&sendfile),
		// TODO 0x0bc => Some(&getpmsg),
		// TODO 0x0bd => Some(&putpmsg),
		0x0be => Some(&vfork),
		// TODO 0x0bf => Some(&ugetrlimit),
		0x0c0 => Some(&mmap2),
		// TODO 0x0c1 => Some(&truncate64),
		// TODO 0x0c2 => Some(&ftruncate64),
		// TODO 0x0c3 => Some(&stat64),
		// TODO 0x0c4 => Some(&lstat64),
		0x0c5 => Some(&fstat64),
		// TODO 0x0c6 => Some(&lchown32),
		0x0c7 => Some(&getuid32),
		0x0c8 => Some(&getgid32),
		0x0c9 => Some(&geteuid32),
		0x0ca => Some(&getegid32),
		// TODO 0x0cb => Some(&setreuid32),
		// TODO 0x0cc => Some(&setregid32),
		// TODO 0x0cd => Some(&getgroups32),
		// TODO 0x0ce => Some(&setgroups32),
		// TODO 0x0cf => Some(&fchown32),
		// TODO 0x0d0 => Some(&setresuid32),
		// TODO 0x0d1 => Some(&getresuid32),
		// TODO 0x0d2 => Some(&setresgid32),
		// TODO 0x0d3 => Some(&getresgid32),
		0x0d4 => Some(&chown32),
		0x0d5 => Some(&setuid32),
		0x0d6 => Some(&setgid32),
		// TODO 0x0d7 => Some(&setfsuid32),
		// TODO 0x0d8 => Some(&setfsgid32),
		// TODO 0x0d9 => Some(&pivot_root),
		// TODO 0x0da => Some(&mincore),
		0x0db => Some(&madvise),
		0x0dc => Some(&getdents64),
		0x0dd => Some(&fcntl64),
		0x0e0 => Some(&gettid),
		// TODO 0x0e1 => Some(&readahead),
		// TODO 0x0e2 => Some(&setxattr),
		// TODO 0x0e3 => Some(&lsetxattr),
		// TODO 0x0e4 => Some(&fsetxattr),
		// TODO 0x0e5 => Some(&getxattr),
		// TODO 0x0e6 => Some(&lgetxattr),
		// TODO 0x0e7 => Some(&fgetxattr),
		// TODO 0x0e8 => Some(&listxattr),
		// TODO 0x0e9 => Some(&llistxattr),
		// TODO 0x0ea => Some(&flistxattr),
		// TODO 0x0eb => Some(&removexattr),
		// TODO 0x0ec => Some(&lremovexattr),
		// TODO 0x0ed => Some(&fremovexattr),
		0x0ee => Some(&tkill),
		// TODO 0x0ef => Some(&sendfile64),
		// TODO 0x0f0 => Some(&futex),
		// TODO 0x0f1 => Some(&sched_setaffinity),
		// TODO 0x0f2 => Some(&sched_getaffinity),
		0x0f3 => Some(&set_thread_area),
		// TODO 0x0f4 => Some(&get_thread_area),
		// TODO 0x0f5 => Some(&io_setup),
		// TODO 0x0f6 => Some(&io_destroy),
		// TODO 0x0f7 => Some(&io_getevents),
		// TODO 0x0f8 => Some(&io_submit),
		// TODO 0x0f9 => Some(&io_cancel),
		// TODO 0x0fa => Some(&fadvise64),
		0x0fc => Some(&exit_group),
		// TODO 0x0fd => Some(&lookup_dcookie),
		// TODO 0x0fe => Some(&epoll_create),
		// TODO 0x0ff => Some(&epoll_ctl),
		// TODO 0x100 => Some(&epoll_wait),
		// TODO 0x101 => Some(&remap_file_pages),
		0x102 => Some(&set_tid_address),
		0x103 => Some(&timer_create),
		0x104 => Some(&timer_settime),
		// TODO 0x105 => Some(&timer_gettime),
		// TODO 0x106 => Some(&timer_getoverrun),
		0x107 => Some(&timer_delete),
		// TODO 0x108 => Some(&clock_settime),
		0x109 => Some(&clock_gettime),
		// TODO 0x10a => Some(&clock_getres),
		// TODO 0x10b => Some(&clock_nanosleep),
		0x10c => Some(&statfs64),
		0x10d => Some(&fstatfs64),
		// TODO 0x10e => Some(&tgkill),
		// TODO 0x10f => Some(&utimes),
		0x110 => Some(&fadvise64_64),
		// TODO 0x111 => Some(&vserver),
		// TODO 0x112 => Some(&mbind),
		// TODO 0x113 => Some(&get_mempolicy),
		// TODO 0x114 => Some(&set_mempolicy),
		// TODO 0x115 => Some(&mq_open),
		// TODO 0x116 => Some(&mq_unlink),
		// TODO 0x117 => Some(&mq_timedsend),
		// TODO 0x118 => Some(&mq_timedreceive),
		// TODO 0x119 => Some(&mq_notify),
		// TODO 0x11a => Some(&mq_getsetattr),
		// TODO 0x11b => Some(&kexec_load),
		// TODO 0x11c => Some(&waitid),
		// TODO 0x11e => Some(&add_key),
		// TODO 0x11f => Some(&request_key),
		// TODO 0x120 => Some(&keyctl),
		// TODO 0x121 => Some(&ioprio_set),
		// TODO 0x122 => Some(&ioprio_get),
		// TODO 0x123 => Some(&inotify_init),
		// TODO 0x124 => Some(&inotify_add_watch),
		// TODO 0x125 => Some(&inotify_rm_watch),
		// TODO 0x126 => Some(&migrate_pages),
		0x127 => Some(&openat),
		// TODO 0x128 => Some(&mkdirat),
		// TODO 0x129 => Some(&mknodat),
		// TODO 0x12a => Some(&fchownat),
		// TODO 0x12b => Some(&futimesat),
		// TODO 0x12c => Some(&fstatat64),
		0x12d => Some(&unlinkat),
		// TODO 0x12e => Some(&renameat),
		0x12f => Some(&linkat),
		0x130 => Some(&symlinkat),
		// TODO 0x131 => Some(&readlinkat),
		0x132 => Some(&fchmodat),
		0x133 => Some(&faccessat),
		0x134 => Some(&pselect6),
		// TODO 0x135 => Some(&ppoll),
		// TODO 0x136 => Some(&unshare),
		// TODO 0x137 => Some(&set_robust_list),
		// TODO 0x138 => Some(&get_robust_list),
		0x139 => Some(&splice),
		// TODO 0x13a => Some(&sync_file_range),
		// TODO 0x13b => Some(&tee),
		// TODO 0x13c => Some(&vmsplice),
		// TODO 0x13d => Some(&move_pages),
		// TODO 0x13e => Some(&getcpu),
		// TODO 0x13f => Some(&epoll_pwait),
		0x140 => Some(&utimensat),
		// TODO 0x141 => Some(&signalfd),
		// TODO 0x142 => Some(&timerfd_create),
		// TODO 0x143 => Some(&eventfd),
		// TODO 0x144 => Some(&fallocate),
		// TODO 0x145 => Some(&timerfd_settime),
		// TODO 0x146 => Some(&timerfd_gettime),
		// TODO 0x147 => Some(&signalfd4),
		// TODO 0x148 => Some(&eventfd2),
		// TODO 0x149 => Some(&epoll_create1),
		// TODO 0x14a => Some(&dup3),
		0x14b => Some(&pipe2),
		// TODO 0x14c => Some(&inotify_init1),
		0x14d => Some(&preadv),
		0x14e => Some(&pwritev),
		// TODO 0x14f => Some(&rt_tgsigqueueinfo),
		// TODO 0x150 => Some(&perf_event_open),
		// TODO 0x151 => Some(&recvmmsg),
		// TODO 0x152 => Some(&fanotify_init),
		// TODO 0x153 => Some(&fanotify_mark),
		0x154 => Some(&prlimit64),
		// TODO 0x155 => Some(&name_to_handle_at),
		// TODO 0x156 => Some(&open_by_handle_at),
		// TODO 0x157 => Some(&clock_adjtime),
		0x158 => Some(&syncfs),
		// TODO 0x159 => Some(&sendmmsg),
		// TODO 0x15a => Some(&setns),
		// TODO 0x15b => Some(&process_vm_readv),
		// TODO 0x15c => Some(&process_vm_writev),
		// TODO 0x15d => Some(&kcmp),
		0x15e => Some(&finit_module),
		// TODO 0x15f => Some(&sched_setattr),
		// TODO 0x160 => Some(&sched_getattr),
		0x161 => Some(&renameat2),
		// TODO 0x162 => Some(&seccomp),
		0x163 => Some(&getrandom),
		// TODO 0x164 => Some(&memfd_create),
		// TODO 0x165 => Some(&bpf),
		// TODO 0x166 => Some(&execveat),
		0x167 => Some(&socket),
		0x168 => Some(&socketpair),
		0x169 => Some(&bind),
		0x16a => Some(&connect),
		// TODO 0x16b => Some(&listen),
		// TODO 0x16c => Some(&accept4),
		0x16d => Some(&getsockopt),
		0x16e => Some(&setsockopt),
		0x16f => Some(&getsockname),
		// TODO 0x170 => Some(&getpeername),
		0x171 => Some(&sendto),
		// TODO 0x172 => Some(&sendmsg),
		// TODO 0x173 => Some(&recvfrom),
		// TODO 0x174 => Some(&recvmsg),
		0x175 => Some(&shutdown),
		// TODO 0x176 => Some(&userfaultfd),
		// TODO 0x177 => Some(&membarrier),
		// TODO 0x178 => Some(&mlock2),
		// TODO 0x179 => Some(&copy_file_range),
		0x17a => Some(&preadv2),
		0x17b => Some(&pwritev2),
		// TODO 0x17c => Some(&pkey_mprotect),
		// TODO 0x17d => Some(&pkey_alloc),
		// TODO 0x17e => Some(&pkey_free),
		0x17f => Some(&statx),
		0x180 => Some(&arch_prctl),
		// TODO 0x181 => Some(&io_pgetevents),
		// TODO 0x182 => Some(&rseq),
		// TODO 0x189 => Some(&semget),
		// TODO 0x18a => Some(&semctl),
		// TODO 0x18b => Some(&shmget),
		// TODO 0x18c => Some(&shmctl),
		// TODO 0x18d => Some(&shmat),
		// TODO 0x18e => Some(&shmdt),
		// TODO 0x18f => Some(&msgget),
		// TODO 0x190 => Some(&msgsnd),
		// TODO 0x191 => Some(&msgrcv),
		// TODO 0x192 => Some(&msgctl),
		0x193 => Some(&clock_gettime64),
		// TODO 0x194 => Some(&clock_settime64),
		// TODO 0x195 => Some(&clock_adjtime64),
		// TODO 0x196 => Some(&clock_getres_time64),
		// TODO 0x197 => Some(&clock_nanosleep_time64),
		// TODO 0x198 => Some(&timer_gettime64),
		// TODO 0x199 => Some(&timer_settime64),
		// TODO 0x19a => Some(&timerfd_gettime64),
		// TODO 0x19b => Some(&timerfd_settime64),
		// TODO 0x19c => Some(&utimensat_time64),
		// TODO 0x19d => Some(&pselect6_time64),
		// TODO 0x19e => Some(&ppoll_time64),
		// TODO 0x1a0 => Some(&io_pgetevents_time64),
		// TODO 0x1a1 => Some(&recvmmsg_time64),
		// TODO 0x1a2 => Some(&mq_timedsend_time64),
		// TODO 0x1a3 => Some(&mq_timedreceive_time64),
		// TODO 0x1a4 => Some(&semtimedop_time64),
		// TODO 0x1a5 => Some(&rt_sigtimedwait_time64),
		// TODO 0x1a6 => Some(&futex_time64),
		// TODO 0x1a7 => Some(&sched_rr_get_interval_time64),
		// TODO 0x1a8 => Some(&pidfd_send_signal),
		// TODO 0x1a9 => Some(&io_uring_setup),
		// TODO 0x1aa => Some(&io_uring_enter),
		// TODO 0x1ab => Some(&io_uring_register),
		// TODO 0x1ac => Some(&open_tree),
		// TODO 0x1ad => Some(&move_mount),
		// TODO 0x1ae => Some(&fsopen),
		// TODO 0x1af => Some(&fsconfig),
		// TODO 0x1b0 => Some(&fsmount),
		// TODO 0x1b1 => Some(&fspick),
		// TODO 0x1b2 => Some(&pidfd_open),
		// TODO 0x1b3 => Some(&clone3),
		// TODO 0x1b4 => Some(&close_range),
		// TODO 0x1b5 => Some(&openat2),
		// TODO 0x1b6 => Some(&pidfd_getfd),
		0x1b7 => Some(&faccessat2),
		// TODO 0x1b8 => Some(&process_madvise),
		// TODO 0x1b9 => Some(&epoll_pwait2),
		// TODO 0x1ba => Some(&mount_setattr),
		// TODO 0x1bb => Some(&quotactl_fd),
		// TODO 0x1bc => Some(&landlock_create_ruleset),
		// TODO 0x1bd => Some(&landlock_add_rule),
		// TODO 0x1be => Some(&landlock_restrict_self),
		// TODO 0x1bf => Some(&memfd_secret),
		// TODO 0x1c0 => Some(&process_mrelease),
		// TODO 0x1c1 => Some(&futex_waitv),
		// TODO 0x1c2 => Some(&set_mempolicy_home_node),
		_ => None,
	}
}

/// This function is called whenever a system call is triggered.
#[no_mangle]
pub extern "C" fn syscall_handler(regs: &mut Regs) {
	let id = regs.eax;
	let result = match get_syscall(id) {
		Some(handler) => (handler)(regs),
		// The system call doesn't exist. Kill the process with SIGSYS
		None => {
			{
				let proc_mutex = Process::current_assert();
				let mut proc = proc_mutex.lock();
				if cfg!(feature = "strace") {
					crate::println!(
						"[strace PID: {}] invalid syscall (ID: 0x{:x})",
						proc.pid,
						id
					);
				}
				// SIGSYS cannot be caught, thus the process will be terminated
				proc.kill_now(&Signal::SIGSYS);
			}
			crate::enter_loop();
		}
	};
	regs.set_syscall_return(result);
}
