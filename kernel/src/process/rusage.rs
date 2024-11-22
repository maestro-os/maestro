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

//! Monitoring of the resource usage of processes.

use crate::{sync::mutex::Mutex, time::unit::Timeval};
use core::sync::atomic::AtomicI64;

// TODO Place calls in kernel's code to update usage

/// Usage of each resource by a process.
#[derive(Debug, Default)]
pub struct Rusage {
	/// User CPU time used.
	pub ru_utime: Mutex<Timeval>,
	/// System CPU time used.
	pub ru_stime: Mutex<Timeval>,
	/// Maximum resident set size.
	pub ru_maxrss: AtomicI64,
	/// Integral shared memory size.
	pub ru_ixrss: AtomicI64,
	/// Integral unshared data size.
	pub ru_idrss: AtomicI64,
	/// Integral unshared stack size.
	pub ru_isrss: AtomicI64,
	/// Page reclaims (soft page faults).
	pub ru_minflt: AtomicI64,
	/// Page faults (hard page faults).
	pub ru_majflt: AtomicI64,
	/// Swaps.
	pub ru_nswap: AtomicI64,
	/// Block input operations.
	pub ru_inblock: AtomicI64,
	/// Block output operations.
	pub ru_oublock: AtomicI64,
	/// IPC messages sent.
	pub ru_msgsnd: AtomicI64,
	/// IPC messages received.
	pub ru_msgrcv: AtomicI64,
	/// Signals received.
	pub ru_nsignals: AtomicI64,
	/// Voluntary context switches.
	pub ru_nvcsw: AtomicI64,
	/// Involuntary context switches.
	pub ru_nivcsw: AtomicI64,
}
