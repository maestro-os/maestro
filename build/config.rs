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

//! This file implements the configuration file for compilation.

use serde::Deserialize;
use std::{fs, io};

/// The debug section of the configuration file.
#[derive(Deserialize)]
struct ConfigDebug {
	/// If enabled, the kernel tests storage.
	///
	/// **Warning**: this option is destructive for any data present on disks connected to the
	/// host.
	storage_test: bool,

	/// If enabled, the kernel is compiled for QEMU. This feature is not *required* for QEMU but
	/// it can provide additional features.
	qemu: bool,

	/// If enabled, the kernel places a magic number in malloc chunks to allow checking integrity.
	malloc_magic: bool,
	/// If enabled, the kernel checks integrity of memory allocations.
	///
	/// **Warning**: this options slows down the system significantly.
	malloc_check: bool,
}

/// The compilation configuration.
#[derive(Deserialize)]
pub struct Config {
	/// Debug section.
	debug: ConfigDebug,
}

impl Config {
	/// Reads the configuration file.
	pub fn read() -> io::Result<Self> {
		const FILE_DEFAULT: &str = "default.build-config.toml";
		const FILE: &str = "build-config.toml";

		println!("cargo:rerun-if-changed={FILE_DEFAULT}");
		println!("cargo:rerun-if-changed={FILE}");

		let config_str = match fs::read_to_string(FILE) {
			Ok(s) => s,
			// Fallback to default configuration file
			Err(e) if e.kind() == io::ErrorKind::NotFound => fs::read_to_string(FILE_DEFAULT)?,
			Err(e) => return Err(e),
		};
		toml::from_str(&config_str)
			.map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
	}

	/// Sets the crate's cfg flags according to the configuration.
	pub fn set_cfg(&self, debug: bool) {
		if debug {
			println!("cargo:rustc-cfg=config_debug_debug");
			if self.debug.storage_test {
				println!("cargo:rustc-cfg=config_debug_storage_test");
			}
			if self.debug.qemu {
				println!("cargo:rustc-cfg=config_debug_qemu");
			}
			if self.debug.malloc_magic {
				println!("cargo:rustc-cfg=config_debug_malloc_magic");
			}
			if self.debug.malloc_check {
				println!("cargo:rustc-cfg=config_debug_malloc_check");
			}
		}
	}
}
