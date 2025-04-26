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

//! The uptime file returns the amount of time elapsed since the system started up.

use crate::{
	file::{fs::FileOps, File},
	format_content,
	memory::user::UserSlice,
	time::clock::{current_time_ns, Clock},
};
use utils::errno::EResult;

/// The `uptime` file.
#[derive(Debug, Default)]
pub struct Uptime;

impl FileOps for Uptime {
	fn read(&self, _file: &File, off: u64, buf: UserSlice<u8>) -> EResult<usize> {
		let uptime = current_time_ns(Clock::Boottime) / 10_000_000;
		let uptime_upper = uptime / 100;
		let uptime_lower = uptime % 100;
		// TODO second value is the total amount of time each core has spent idle
		format_content!(off, buf, "{uptime_upper}.{uptime_lower:02} 0.00\n")
	}
}
