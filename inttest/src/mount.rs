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

//! Filesystem mounting tests.

use crate::{log, util, util::TestResult};
use std::{ffi::CString, fs, ptr::null};

pub fn mount(src: &str, target: &str, fstype: &str) -> TestResult {
	log!("Create directory");
	fs::create_dir_all(target)?;
	log!("Mount");
	let src = CString::new(src)?;
	let target = CString::new(target)?;
	let fstype = CString::new(fstype)?;
	util::mount(
		src.as_c_str(),
		target.as_c_str(),
		fstype.as_c_str(),
		0,
		null(),
	)?;
	Ok(())
}

pub fn umount(target: &str) -> TestResult {
	let target = CString::new(target)?;
	util::umount(target.as_c_str())?;
	Ok(())
}
