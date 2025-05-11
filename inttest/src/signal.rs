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

//! Signals testing.

use crate::{
	log,
	util::{TestResult, kill, signal},
};
use libc::{SIG_DFL, SIGINT, getpid};
use std::{
	ffi::c_int,
	sync::atomic::{
		AtomicBool,
		Ordering::{Acquire, Release},
	},
};

static HIT: AtomicBool = AtomicBool::new(false);

extern "C" fn signal_handler(_: c_int) {
	HIT.store(true, Release);
}

pub fn handler() -> TestResult {
	log!("Register signal handler");
	signal(SIGINT, signal_handler as usize)?;

	log!("Kill self");
	assert!(!HIT.load(Acquire));
	unsafe {
		kill(getpid(), SIGINT)?;
	}
	assert!(HIT.load(Acquire));

	log!("Cleanup");
	HIT.store(false, Release);
	signal(SIGINT, SIG_DFL)?;

	Ok(())
}
