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

//! Kernel module testing.

use crate::{
	log, test_assert, test_assert_eq,
	util::{TestResult, delete_module, finit_module},
};
use std::{
	fs,
	fs::File,
	io,
	os::{
		fd::AsRawFd,
		unix::fs::{FileTypeExt, MetadataExt},
	},
};

pub fn load() -> TestResult {
	log!("Load the module");
	let file = File::open("/mod.kmod")?;
	finit_module(file.as_raw_fd())?;
	drop(file);

	log!("Check presence of the device file");
	let stat = fs::metadata("/dev/test")?;
	test_assert!(stat.file_type().is_char_device());
	test_assert_eq!(stat.rdev(), libc::makedev(255, 255));

	Ok(())
}

pub fn unload() -> TestResult {
	log!("Unload the module");
	delete_module(c"inttest")?;

	log!("Check the device file is gone");
	let res = fs::metadata("/dev/test");
	test_assert!(matches!(res, Err(e) if e.kind() == io::ErrorKind::NotFound));

	Ok(())
}
