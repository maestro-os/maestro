//! This module implements the status file, which allows to retrieve the current status of the
//! process.

use crate::errno::Errno;
use crate::file::fs::kernfs::node::KernFSNode;
use crate::file::mountpoint;
use crate::file::FileContent;
use crate::file::Gid;
use crate::file::Mode;
use crate::file::Uid;
use crate::process::pid::Pid;
use crate::process::Process;
use crate::util::container::string::String;
use crate::util::io::IO;
use crate::util::ptr::cow::Cow;
use core::cmp::min;

/// Structure representing the mount node of the procfs.
pub struct Status {
	/// The PID of the process.
	pub pid: Pid,
}

impl KernFSNode for Status {
	fn get_mode(&self) -> Mode {
		0o444
	}

	fn get_uid(&self) -> Uid {
		let proc_mutex = Process::get_by_pid(self.pid).unwrap();
		let proc_guard = proc_mutex.lock();
		let proc = proc_guard.get();

		proc.get_euid()
	}

	fn get_gid(&self) -> Gid {
		let proc_mutex = Process::get_by_pid(self.pid).unwrap();
		let proc_guard = proc_mutex.lock();
		let proc = proc_guard.get();

		proc.get_egid()
	}

	fn get_content<'a>(&'a self) -> Cow<'a, FileContent> {
		Cow::from(FileContent::Regular)
	}
}

impl IO for Status {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&mut self, offset: u64, buff: &mut [u8]) -> Result<(u64, bool), Errno> {
		if buff.is_empty() {
			return Ok((0, false));
		}

		// Generating content
		let mut content = String::new();
		// TODO example:
		/*Name:   cat
		Umask:  0022
		State:  R (running)
		Tgid:   2175
		Ngid:   0
		Pid:    2175
		PPid:   2172
		TracerPid:      0
		Uid:    1000    1000    1000    1000
		Gid:    1000    1000    1000    1000
		FDSize: 256
		Groups: 1000 
		NStgid: 2175
		NSpid:  2175
		NSpgid: 2175
		NSsid:  2172
		VmPeak:     5796 kB
		VmSize:     5796 kB
		VmLck:         0 kB
		VmPin:         0 kB
		VmHWM:       900 kB
		VmRSS:       900 kB
		RssAnon:              88 kB
		RssFile:             812 kB
		RssShmem:              0 kB
		VmData:      360 kB
		VmStk:       132 kB
		VmExe:        20 kB
		VmLib:      1668 kB
		VmPTE:        48 kB
		VmSwap:        0 kB
		HugetlbPages:          0 kB
		CoreDumping:    0
		THP_enabled:    1
		Threads:        1
		SigQ:   1/31244
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
		NoNewPrivs:     0
		Seccomp:        0
		Seccomp_filters:        0
		Speculation_Store_Bypass:       thread vulnerable
		SpeculationIndirectBranch:      conditional enabled
		Cpus_allowed:   ff
		Cpus_allowed_list:      0-7
		Mems_allowed:   00000001
		Mems_allowed_list:      0
		voluntary_ctxt_switches:        0
		nonvoluntary_ctxt_switches:     0*/

		// Copying content to userspace buffer
		let content_bytes = content.as_bytes();
		let len = min((content_bytes.len() as u64 - offset) as usize, buff.len());
		buff[..len].copy_from_slice(&content_bytes[(offset as usize)..(offset as usize + len)]);

		let eof = (offset + len as u64) >= content_bytes.len() as u64;
		Ok((len as _, eof))
	}

	fn write(&mut self, _offset: u64, _buff: &[u8]) -> Result<u64, Errno> {
		Err(errno!(EINVAL))
	}

	fn poll(&mut self, _mask: u32) -> Result<u32, Errno> {
		// TODO
		todo!();
	}
}
