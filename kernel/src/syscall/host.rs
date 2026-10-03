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

//! Host management system calls.

use crate::{
	NAME, VERSION,
	arch::ARCH,
	file::perm::is_privileged,
	memory::{
		stats::MEM_INFO,
		user::{UserPtr, UserSlice},
	},
	power,
	process::{PROCESS_FLAG_LINUX, PROCESSES, Process},
	time::clock::{Clock, current_time_sec},
};
use core::{
	ffi::{c_char, c_int, c_uint, c_ulong, c_ushort, c_void},
	hint::unlikely,
	sync::atomic::Ordering::Acquire,
};
use utils::{errno, errno::EResult, limits::HOST_NAME_MAX, slice_copy};

/// The length of a field of the utsname structure.
const UTSNAME_LENGTH: usize = 65;

/// First magic number.
const MAGIC: c_int = 0xde145e83u32 as _;
/// Second magic number.
const MAGIC2: c_int = 0x40367d6eu32 as _;

/// Command to power off the system.
const CMD_POWEROFF: c_int = 0;
/// Command to reboot the system.
const CMD_REBOOT: c_int = 1;
/// Command to halt the system.
const CMD_HALT: c_int = 2;
/// Command to suspend the system.
const CMD_SUSPEND: c_int = 3;

/// Userspace structure storing uname information.
#[derive(Debug)]
#[repr(C)]
pub struct Utsname {
	/// Operating system name.
	sysname: [u8; UTSNAME_LENGTH],
	/// Network node hostname.
	nodename: [u8; UTSNAME_LENGTH],
	/// Operating system release.
	release: [u8; UTSNAME_LENGTH],
	/// Operating system version.
	version: [u8; UTSNAME_LENGTH],
	/// Hardware identifier.
	machine: [u8; UTSNAME_LENGTH],
}

pub fn uname(buf: UserPtr<Utsname>) -> EResult<usize> {
	let linux = Process::current().flags.load(Acquire) & PROCESS_FLAG_LINUX != 0;
	let sysname = if linux { b"Linux" } else { NAME.as_bytes() };
	let mut utsname = Utsname {
		sysname: [0; UTSNAME_LENGTH],
		nodename: [0; UTSNAME_LENGTH],
		release: [0; UTSNAME_LENGTH],
		version: [0; UTSNAME_LENGTH],
		machine: [0; UTSNAME_LENGTH],
	};
	slice_copy(sysname, &mut utsname.sysname);
	slice_copy(&crate::HOSTNAME.lock(), &mut utsname.nodename);
	slice_copy(VERSION.as_bytes(), &mut utsname.release);
	slice_copy(&[], &mut utsname.version);
	slice_copy(ARCH.as_bytes(), &mut utsname.machine);
	buf.copy_to_user(&utsname)?;
	Ok(0)
}

/// The size of the padding at the end of `struct sysinfo`, which the ABI defines as
/// `20 - 2 * sizeof(long) - sizeof(int)`.
const SYSINFO_PAD: usize = 20 - 2 * size_of::<c_ulong>() - size_of::<c_uint>();

/// Userspace structure storing some system usage statistics.
#[derive(Debug)]
#[repr(C)]
pub struct Sysinfo {
	/// Seconds since boot
	uptime: c_ulong,
	/// 1, 5 and 15 minute load averages
	loads: [c_ulong; 3],
	/// Total usable main memory size
	totalram: c_ulong,
	/// Available memory size
	freeram: c_ulong,
	/// Amount of shared memory
	sharedram: c_ulong,
	/// Memory used by buffers
	bufferram: c_ulong,
	/// Total swap space size
	totalswap: c_ulong,
	/// Swap space still available
	freeswap: c_ulong,
	/// Number of current processes
	procs: c_ushort,
	/// Padding
	pad: c_ushort,
	/// Total high memory size
	totalhigh: c_ulong,
	/// Available high memory size
	freehigh: c_ulong,
	/// Memory unit size in bytes
	mem_unit: c_uint,
	_f: [c_char; SYSINFO_PAD],
}

/// 32-bit version of [`Sysinfo`].
#[derive(Debug)]
#[repr(C)]
pub struct Sysinfo32 {
	/// Seconds since boot
	uptime: i32,
	/// 1, 5 and 15 minute load averages
	loads: [u32; 3],
	/// Total usable main memory size
	totalram: u32,
	/// Available memory size
	freeram: u32,
	/// Amount of shared memory
	sharedram: u32,
	/// Memory used by buffers
	bufferram: u32,
	/// Total swap space size
	totalswap: u32,
	/// Swap space still available
	freeswap: u32,
	/// Number of current processes
	procs: u16,
	/// Padding
	pad: u16,
	/// Total high memory size
	totalhigh: u32,
	/// Available high memory size
	freehigh: u32,
	/// Memory unit size in bytes
	mem_unit: u32,
	_f: [c_char; 20 - 2 * size_of::<u32>() - size_of::<u32>()],
}

// The ABI fixes both layouts, and getting them wrong overflows a userspace buffer
const _: () = assert!(size_of::<Sysinfo32>() == 64);
#[cfg(target_arch = "x86_64")]
const _: () = assert!(size_of::<Sysinfo>() == 112);
#[cfg(target_arch = "x86")]
const _: () = assert!(size_of::<Sysinfo>() == 64);

impl From<&Sysinfo> for Sysinfo32 {
	fn from(info: &Sysinfo) -> Self {
		Self {
			uptime: info.uptime as _,
			loads: [info.loads[0] as _, info.loads[1] as _, info.loads[2] as _],
			totalram: info.totalram as _,
			freeram: info.freeram as _,
			sharedram: info.sharedram as _,
			bufferram: info.bufferram as _,
			totalswap: info.totalswap as _,
			freeswap: info.freeswap as _,
			procs: info.procs,
			pad: 0,
			totalhigh: info.totalhigh as _,
			freehigh: info.freehigh as _,
			mem_unit: info.mem_unit,
			_f: [0; 20 - 2 * size_of::<u32>() - size_of::<u32>()],
		}
	}
}

fn get_sysinfo() -> Sysinfo {
	let mem_info = MEM_INFO.lock().clone();
	Sysinfo {
		uptime: current_time_sec(Clock::Boottime) as _,
		loads: [0; 3], // TODO
		totalram: mem_info.mem_total as _,
		freeram: mem_info.mem_free as _,
		sharedram: 0, // TODO
		bufferram: 0, // TODO
		totalswap: 0, // TODO
		freeswap: 0,  // TODO
		procs: PROCESSES.read().len() as _,
		pad: 0,
		totalhigh: 0, // TODO
		freehigh: 0,  // TODO
		mem_unit: 1024,
		_f: [0; SYSINFO_PAD],
	}
}

pub fn sysinfo(info: UserPtr<Sysinfo>) -> EResult<usize> {
	info.copy_to_user(&get_sysinfo())?;
	Ok(0)
}

pub fn sysinfo32(info: UserPtr<Sysinfo32>) -> EResult<usize> {
	info.copy_to_user(&Sysinfo32::from(&get_sysinfo()))?;
	Ok(0)
}

pub fn sethostname(name: *mut u8, len: usize) -> EResult<usize> {
	// Check the size of the hostname is in bounds
	if unlikely(len > HOST_NAME_MAX) {
		return Err(errno!(EINVAL));
	}
	// Check permission
	if unlikely(!is_privileged()) {
		return Err(errno!(EPERM));
	}
	// Copy
	let name = UserSlice::from_user(name, len)?;
	let new_hostname = name.copy_from_user_vec(0)?.ok_or(errno!(EFAULT))?;
	*crate::HOSTNAME.lock() = new_hostname;
	Ok(0)
}

pub fn reboot(magic: c_int, magic2: c_int, cmd: c_int, _arg: *const c_void) -> EResult<usize> {
	// Validation
	if magic != MAGIC || magic2 != MAGIC2 {
		return Err(errno!(EINVAL));
	}
	if unlikely(!is_privileged()) {
		return Err(errno!(EPERM));
	}
	// Debug commands: shutdown with QEMU
	#[cfg(config_debug_qemu)]
	{
		use crate::debug::qemu;
		match cmd {
			-1 => qemu::exit(qemu::SUCCESS),
			-2 => qemu::exit(qemu::FAILURE),
			_ => {}
		}
	}
	match cmd {
		CMD_POWEROFF => power::shutdown(),
		CMD_REBOOT => power::reboot(),
		CMD_HALT => power::halt(),
		CMD_SUSPEND => {
			// TODO Use ACPI to suspend the system
			todo!()
		}
		_ => Err(errno!(EINVAL)),
	}
}
