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

// TODO Place calls in kernel's code to update usage

/// Usage of each resource by a process.
#[derive(Clone, Debug, Default)]
pub struct Rusage {
	/// User CPU time used.
	pub ru_utime: Timeval,
	/// System CPU time used.
	pub ru_stime: Timeval,
	/// Maximum resident set size.
	pub ru_maxrss: i64,
	/// Integral shared memory size.
	pub ru_ixrss: i64,
	/// Integral unshared data size.
	pub ru_idrss: i64,
	/// Integral unshared stack size.
	pub ru_isrss: i64,
	/// Page reclaims (soft page faults).
	pub ru_minflt: i64,
	/// Page faults (hard page faults).
	pub ru_majflt: i64,
	/// Swaps.
	pub ru_nswap: i64,
	/// Block input operations.
	pub ru_inblock: i64,
	/// Block output operations.
	pub ru_oublock: i64,
	/// IPC messages sent.
	pub ru_msgsnd: i64,
	/// IPC messages received.
	pub ru_msgrcv: i64,
	/// Signals received.
	pub ru_nsignals: i64,
	/// Voluntary context switches.
	pub ru_nvcsw: i64,
	/// Involuntary context switches.
	pub ru_nivcsw: i64,
}
