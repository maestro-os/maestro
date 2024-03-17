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

use crate::time::unit::Timeval;

/// Usage of each resource by a process.
#[derive(Clone, Default, Debug)]
pub struct RUsage {
	/// User CPU time used.
	pub ru_utime: Timeval,
	/// System CPU time used.
	pub ru_stime: Timeval,
	/// Maximum resident set size.
	pub ru_maxrss: i32,
	/// Integral shared memory size.
	pub ru_ixrss: i32,
	/// Integral unshared data size.
	pub ru_idrss: i32,
	/// Integral unshared stack size.
	pub ru_isrss: i32,
	/// Page reclaims (soft page faults).
	pub ru_minflt: i32,
	/// Page faults (hard page faults).
	pub ru_majflt: i32,
	/// Swaps.
	pub ru_nswap: i32,
	/// Block input operations.
	pub ru_inblock: i32,
	/// Block output operations.
	pub ru_oublock: i32,
	/// IPC messages sent.
	pub ru_msgsnd: i32,
	/// IPC messages received.
	pub ru_msgrcv: i32,
	/// Signals received.
	pub ru_nsignals: i32,
	/// Voluntary context switches.
	pub ru_nvcsw: i32,
	/// Involuntary context switches.
	pub ru_nivcsw: i32,
}

// TODO Place calls in kernel's code to update usage
