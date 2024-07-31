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

//! Boot stub for integration tests.
//!
//! This file exists to run the tests as a second process in order to retrieve the exit code, then
//! shutdown the machine.

use std::process::Command;

pub fn main() {
	let status = Command::new("/inttest").status().unwrap();
	let cmd = if status.success() { -1 } else { -2 };
	// Shutdown
	unsafe {
		libc::syscall(libc::SYS_reboot, 0xde145e83u32, 0x40367d6eu32, cmd, 0);
	}
	panic!("Shutdown failed!");
}
