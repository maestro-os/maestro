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

//! Integration test module, to check loading and unloading works.

#![no_std]
#![no_main]

#[no_link]
extern crate kernel;

kernel::module!([]);

use kernel::{
	device::{CharDev, DeviceID},
	file::fs::DummyOps,
	utils::{collections::path::PathBuf, ptr::arc::Arc},
};

static mut DEV: Option<Arc<CharDev>> = None;

#[unsafe(no_mangle)]
pub extern "C" fn init() -> bool {
	kernel::println!("Module loaded");
	let dev = CharDev::new(
		DeviceID {
			major: u32::MAX,
			minor: u32::MAX,
		},
		PathBuf::try_from(b"/dev/test").unwrap(),
		0o777,
		DummyOps,
	)
	.unwrap();
	unsafe {
		DEV = Some(dev);
	}
	true
}

#[unsafe(no_mangle)]
pub extern "C" fn fini() {
	unsafe {
		DEV = None;
	}
	kernel::println!("Module unloaded");
}
