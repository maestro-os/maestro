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

//! `clock_gettime64` is like `clock_gettime` but using 64 bits.

use crate::{
	process::{mem_space::copy::SyscallPtr, Process},
	syscall::Args,
	time::{
		clock,
		unit::{ClockIdT, Timespec},
	},
};
use utils::{
	errno,
	errno::{EResult, Errno},
};

pub fn clock_gettime64(
	Args((clockid, tp)): Args<(ClockIdT, SyscallPtr<Timespec>)>,
) -> EResult<usize> {
	let curr_time = clock::current_time_struct::<Timespec>(clockid)?;
	tp.copy_to_user(curr_time)?;
	Ok(0)
}
