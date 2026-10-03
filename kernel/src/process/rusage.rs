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

use crate::time::unit::{Timeval, Timeval32};

// TODO Place calls in kernel's code to update usage

/// Usage of each resource by a process.
#[allow(missing_docs)]
#[derive(Clone, Debug, Default)]
#[repr(C)]
pub struct Rusage {
	pub ru_utime: Timeval,
	pub ru_stime: Timeval,
	pub ru_maxrss: i64,
	pub ru_ixrss: i64,
	pub ru_idrss: i64,
	pub ru_isrss: i64,
	pub ru_minflt: i64,
	pub ru_majflt: i64,
	pub ru_nswap: i64,
	pub ru_inblock: i64,
	pub ru_oublock: i64,
	pub ru_msgsnd: i64,
	pub ru_msgrcv: i64,
	pub ru_nsignals: i64,
	pub ru_nvcsw: i64,
	pub ru_nivcsw: i64,
}

/// 32-bit version of [`Rusage`].
#[allow(missing_docs)]
#[derive(Clone, Debug, Default)]
#[repr(C)]
pub struct Rusage32 {
	pub ru_utime: Timeval32,
	pub ru_stime: Timeval32,
	pub ru_maxrss: i32,
	pub ru_ixrss: i32,
	pub ru_idrss: i32,
	pub ru_isrss: i32,
	pub ru_minflt: i32,
	pub ru_majflt: i32,
	pub ru_nswap: i32,
	pub ru_inblock: i32,
	pub ru_oublock: i32,
	pub ru_msgsnd: i32,
	pub ru_msgrcv: i32,
	pub ru_nsignals: i32,
	pub ru_nvcsw: i32,
	pub ru_nivcsw: i32,
}

impl From<&Rusage> for Rusage32 {
	fn from(r: &Rusage) -> Self {
		Self {
			ru_utime: Timeval32 {
				tv_sec: r.ru_utime.tv_sec as _,
				tv_usec: r.ru_utime.tv_usec as _,
			},
			ru_stime: Timeval32 {
				tv_sec: r.ru_stime.tv_sec as _,
				tv_usec: r.ru_stime.tv_usec as _,
			},
			ru_maxrss: r.ru_maxrss as _,
			ru_ixrss: r.ru_ixrss as _,
			ru_idrss: r.ru_idrss as _,
			ru_isrss: r.ru_isrss as _,
			ru_minflt: r.ru_minflt as _,
			ru_majflt: r.ru_majflt as _,
			ru_nswap: r.ru_nswap as _,
			ru_inblock: r.ru_inblock as _,
			ru_oublock: r.ru_oublock as _,
			ru_msgsnd: r.ru_msgsnd as _,
			ru_msgrcv: r.ru_msgrcv as _,
			ru_nsignals: r.ru_nsignals as _,
			ru_nvcsw: r.ru_nvcsw as _,
			ru_nivcsw: r.ru_nivcsw as _,
		}
	}
}
