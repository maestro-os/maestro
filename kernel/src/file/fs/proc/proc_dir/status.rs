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
	file::{File, fs::FileOps},
	format_content,
	memory::user::UserSlice,
	process::{Process, pid::Pid},
};
use core::{fmt, sync::atomic::Ordering::Acquire};
use utils::{DisplayableStr, errno, errno::EResult};

/// The `status` node of the proc.
#[derive(Debug)]
pub struct Status(pub Pid);

impl FileOps for Status {
	fn read(&self, _file: &File, off: u64, buf: UserSlice<u8>) -> EResult<usize> {
		let proc = Process::get_by_pid(self.0).ok_or_else(|| errno!(ENOENT))?;
		let disp = fmt::from_fn(|f| {
			let name = proc
				.mem_space_opt()
				.as_ref()
				.map(|m| m.exe_info.exe.name.as_bytes())
				.unwrap_or_default();
			let umask = proc.umask.load(Acquire);
			let state = proc.get_state();
			let ap = proc.fs.lock().ap;
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
				name = DisplayableStr(name),
				state_char = state.as_char(),
				state_name = state.as_str(),
				pid = self.0,
				ppid = proc.get_parent_pid(),
				uid = ap.uid,
				euid = ap.euid,
				suid = ap.suid,
				ruid = ap.uid,
				gid = ap.gid,
				egid = ap.egid,
				sgid = ap.sgid,
				rgid = ap.gid,
			)
		});
		format_content!(off, buf, "{disp}")
	}
}
