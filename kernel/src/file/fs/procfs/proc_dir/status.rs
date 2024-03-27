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

//! This module implements the status file, which allows to retrieve the current
//! status of the process.

use crate::{
	file::{
		fs::kernfs::{content::KernFSContent, node::KernFSNode},
		perm::{Gid, Uid},
		Mode,
	},
	process::{pid::Pid, Process},
};
use core::cmp::min;
use utils::{errno, errno::EResult, format, io::IO};

/// Structure representing the status node of the procfs.
#[derive(Debug)]
pub struct Status {
	/// The PID of the process.
	pub pid: Pid,
}

impl KernFSNode for Status {
	fn get_mode(&self) -> Mode {
		0o444
	}

	fn get_uid(&self) -> Uid {
		if let Some(proc_mutex) = Process::get_by_pid(self.pid) {
			proc_mutex.lock().access_profile.get_euid()
		} else {
			0
		}
	}

	fn get_gid(&self) -> Gid {
		if let Some(proc_mutex) = Process::get_by_pid(self.pid) {
			proc_mutex.lock().access_profile.get_egid()
		} else {
			0
		}
	}

	fn get_content(&mut self) -> EResult<KernFSContent<'_>> {
		Ok(FileContent::Regular.into())
	}
}

impl IO for Status {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&mut self, offset: u64, buff: &mut [u8]) -> EResult<(u64, bool)> {
		if buff.is_empty() {
			return Ok((0, false));
		}

		let proc_mutex = Process::get_by_pid(self.pid).ok_or_else(|| errno!(ENOENT))?;
		let proc = proc_mutex.lock();

		let name = proc
			.argv
			.iter()
			.map(|name| unsafe { name.as_str_unchecked() })
			.next()
			.unwrap_or("?");
		let state = proc.get_state();

		// TODO Fill every fields with process's data
		// Generating content
		let content = format!(
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
nonvoluntary_ctxt_switches: 0
",
			umask = proc.umask,
			state_char = state.get_char(),
			state_name = state.as_str(),
			pid = proc.pid,
			ppid = proc.get_parent_pid(),
			uid = proc.access_profile.get_uid(),
			euid = proc.access_profile.get_euid(),
			suid = proc.access_profile.get_suid(),
			ruid = 0, // TODO
			gid = proc.access_profile.get_gid(),
			egid = proc.access_profile.get_egid(),
			sgid = proc.access_profile.get_sgid(),
			rgid = 0, // TODO
		)?;

		// Copying content to userspace buffer
		let content_bytes = content.as_bytes();
		let len = min((content_bytes.len() as u64 - offset) as usize, buff.len());
		buff[..len].copy_from_slice(&content_bytes[(offset as usize)..(offset as usize + len)]);

		let eof = (offset + len as u64) >= content_bytes.len() as u64;
		Ok((len as _, eof))
	}

	fn write(&mut self, _offset: u64, _buff: &[u8]) -> EResult<u64> {
		Err(errno!(EINVAL))
	}

	fn poll(&mut self, _mask: u32) -> EResult<u32> {
		// TODO
		todo!();
	}
}
