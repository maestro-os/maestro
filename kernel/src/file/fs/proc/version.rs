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

//! The `version` file returns the version of the kernel.

use crate::{
	file::{File, fs::FileOps},
	format_content,
	memory::user::UserSlice,
};
use utils::errno::EResult;

/// Kernel version file.
#[derive(Debug, Default)]
pub struct Version;

impl FileOps for Version {
	fn read(&self, _file: &File, off: u64, buf: UserSlice<u8>) -> EResult<usize> {
		format_content!(off, buf, "{} version {}\n", crate::NAME, crate::VERSION)
	}
}
