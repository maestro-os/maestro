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

//! procfs filesystem testing.

use crate::{
	test_assert_eq,
	util::{TestError, TestResult},
};
use std::{collections::HashMap, env, env::current_dir, fs, os::unix::ffi::OsStrExt};

pub fn cwd() -> TestResult {
	let cwd = fs::read_link("/proc/self/cwd")?;
	test_assert_eq!(cwd, current_dir()?);
	Ok(())
}

pub fn exe() -> TestResult {
	let exe = fs::read_link("/proc/self/exe")?;
	test_assert_eq!(exe.as_os_str().as_bytes(), b"/inttest");
	Ok(())
}

pub fn cmdline() -> TestResult {
	let args0 = fs::read("/proc/self/cmdline")?;
	let args1 = env::args_os();
	for (a0, a1) in args0.split(|b| *b == b'\0').zip(args1) {
		test_assert_eq!(a0, a1.as_bytes());
	}
	Ok(())
}

pub fn environ() -> TestResult {
	let environ = fs::read("/proc/self/environ")?;
	let args0 = environ
		.split(|b| *b == b'\0')
		.filter(|var| !var.is_empty())
		.map(|var| {
			let off = var
				.iter()
				.enumerate()
				.find(|(_, b)| **b == b'=')
				.map(|(i, _)| i)
				.ok_or_else(|| TestError("missing `=` for environment variable".to_owned()))?;
			let (name, value) = var.split_at(off);
			Ok((name, &value[1..]))
		})
		.collect::<Result<HashMap<_, _>, TestError>>()?;
	let args1: Vec<_> = env::vars_os().collect();
	let args1 = args1
		.iter()
		.map(|(name, val)| (name.as_bytes(), val.as_bytes()))
		.collect::<HashMap<_, _>>();
	test_assert_eq!(args0, args1);
	Ok(())
}
