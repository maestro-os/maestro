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

//! This kernel module implements a driver for the Intel e1000 ethernet controllers.

#![cfg_attr(not(maestro_builtin), no_std)]
#![cfg_attr(not(maestro_builtin), no_main)]
#![cfg_attr(not(maestro_builtin), feature(likely_unlikely))]
#![allow(unused)] // TODO remove

#[cfg(not(maestro_builtin))]
#[no_link]
extern crate kernel;

// FIXME
//mod driver;
mod nic;

kernel::module!([]);

/// Called on module load
#[cfg_attr(not(maestro_builtin), unsafe(no_mangle))]
pub extern "C" fn init() -> bool {
	// FIXME: driver not yet implemented
	//kernel::device::driver::register(E1000Driver::new()).is_ok()
	false
}

/// Called on module unload
#[cfg_attr(not(maestro_builtin), unsafe(no_mangle))]
pub extern "C" fn fini() {
	// FIXME
	//kernel::device::driver::unregister("e1000");
}
