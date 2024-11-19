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

//! Implementation of the `status` file, which allows to retrieve the current
//! status of the process.

use crate::{
	file::{
		fs::{proc::get_proc_owner, NodeOps},
		FileLocation, FileType, Stat,
	},
	format_content,
	process::{pid::Pid, Process},
};
use core::fmt;
use utils::{errno, errno::EResult};

/// The `status` node of the proc.
#[derive(Debug)]
pub struct Status(Pid);

impl From<Pid> for Status {
	fn from(pid: Pid) -> Self {
		Self(pid)
	}
}

impl NodeOps for Status {
	fn get_stat(&self, _loc: &FileLocation) -> EResult<Stat> {
		let (uid, gid) = get_proc_owner(self.0);
		Ok(Stat {
			mode: FileType::Regular.to_mode() | 0o444,
			uid,
			gid,
			..Default::default()
		})
	}

	fn read_content(&self, _loc: &FileLocation, off: u64, buf: &mut [u8]) -> EResult<usize> {
		let proc = Process::get_by_pid(self.0).ok_or_else(|| errno!(ENOENT))?;
		let mem_space = proc.mem_space.as_ref().unwrap().lock();
		let disp = fmt::from_fn(|f| {
			let state = proc.get_state();
			let fs = proc.fs.lock();
			// TODO Fill every fields with process's data
			writeln!(
				f,
				"Name: {name}
Umask: {umask:4o}
State: {state_char} ({state_name})
Tgid: 0
Ngid: 0
Pid: {pid}
PPid: {ppid}
TracerPid: 0
Uid: {uid} {euid} {suid} {ruid}
Gid: {gid} {egid} {sgid} {rgid}
FDSize: TODO
Groups: TODO
NStgid: TODO
NSpid: TODO
NSpgid: TODO
NSsid: TODO
VmPeak: TODO kB
VmSize: TODO kB
VmLck: TODO kB
VmPin: TODO kB
VmHWM: TODO kB
VmRSS: TODO kB
RssAnon: TODO kB
RssFile: TODO kB
RssShmem: TODO kB
VmData: TODO kB
VmStk: TODO kB
VmExe: TODO kB
VmLib: TODO kB
VmPTE: TODO kB
VmSwap: TODO kB
HugetlbPages: TODO kB
CoreDumping: TODO
THP_enabled: TODO
Threads: TODO
SigQ: TODO/TODO
SigPnd: 0000000000000000
ShdPnd: 0000000000000000
SigBlk: 0000000000000000
SigIgn: 0000000000000000
SigCgt: 0000000000000000
CapInh: 0000000000000000
CapPrm: 0000000000000000
CapEff: 0000000000000000
CapBnd: 000001ffffffffff
CapAmb: 0000000000000000
NoNewPrivs: 0
Seccomp: 0
Seccomp_filters: 0
Speculation_Store_Bypass: thread vulnerable
SpeculationIndirectBranch: conditional enabled
Cpus_allowed: ff
Cpus_allowed_list: 0-7
Mems_allowed: 00000001
Mems_allowed_list: 0
voluntary_ctxt_switches: 0
nonvoluntary_ctxt_switches: 0",
				name = mem_space.exe_info.exe.name,
				umask = fs.umask(),
				state_char = state.as_char(),
				state_name = state.as_str(),
				pid = self.0,
				ppid = proc.get_parent_pid(),
				uid = fs.access_profile.uid,
				euid = fs.access_profile.euid,
				suid = fs.access_profile.suid,
				ruid = fs.access_profile.uid,
				gid = fs.access_profile.gid,
				egid = fs.access_profile.egid,
				sgid = fs.access_profile.sgid,
				rgid = fs.access_profile.gid,
			)
		});
		format_content!(off, buf, "{disp}")
	}
}
