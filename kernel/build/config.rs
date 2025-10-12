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
use std::{
	any::type_name,
	fs,
	io::{self, Write},
	process::exit,
};

/// Build a config name from a stringified expression path.
/// Converts paths like "self.debug.malloc_magic" to "config_debug_malloc_magic".
fn build_cfg_name(full_path: &str) -> String {
	let mut name = String::with_capacity(full_path.len() + 2); // len("config") - len("self") == 2
	name.push_str("config");

	for part in full_path
		.trim_start_matches('&')
		.split('.')
		.skip_while(|s| *s == "self")
	{
		name.push('_');
		name.push_str(part);
	}

	name
}

/// Generate a cfg flag if the value is true.
macro_rules! generate_cfg_flag {
	($value:expr) => {
		if $value {
			let full_path = stringify!($value);
			let cfg_name = build_cfg_name(full_path);
			println!("cargo:rustc-cfg={}", cfg_name);
		}
	};
}

/// Generate a Rust const file in OUT_DIR.
macro_rules! generate_const_file {
	($value:expr) => {{
		fn inner<T: std::fmt::Debug + std::fmt::Display>(value: T, name: &str) {
			let out_dir = std::env::var_os("OUT_DIR").unwrap_or_else(|| {
				eprintln!("OUT_DIR environment variable not set");
				exit(1);
			});
			let dest_path = std::path::Path::new(&out_dir).join(format!("{name}.rs"));

			let mut file = std::fs::File::create(&dest_path).unwrap_or_else(|e| {
				eprintln!("Failed to create file {dest_path:?}: {e}");
				exit(1);
			});

			let write_result = match type_name::<T>() {
				"alloc::string::String" | "&str" | "&alloc::string::String" => {
					write!(file, "{value:?}") // Add quotes around strings
				}
				_ => write!(file, "{value}"),
			};

			write_result.unwrap_or_else(|e| {
				eprintln!("Failed to write to file {dest_path:?}: {e}");
				exit(1);
			});
		}

		let full_path = stringify!($value);
		let file_name = build_cfg_name(full_path);
		inner($value, &file_name);
	}};
}

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

/// The memory management section of the configuration file.
#[derive(Deserialize)]
struct ConfigMemory {
	/// The timeout, in milliseconds, after which a dirty page may be written back to disk.
	writeback_timeout: u64,
}

/// The compilation configuration.
#[derive(Deserialize)]
pub struct Config {
	/// Debug section.
	debug: ConfigDebug,
	/// Memory management section.
	memory: ConfigMemory,
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
		toml::from_str(&config_str).map_err(|e| io::Error::other(e.to_string()))
	}

	/// Sets the crate's cfg flags and generates the const files according to the configuration.
	pub fn set_cfg(&self, debug: bool) {
		if debug {
			generate_cfg_flag!(self.debug.storage_test);
			generate_cfg_flag!(self.debug.qemu);
			generate_cfg_flag!(self.debug.malloc_magic);
			generate_cfg_flag!(self.debug.malloc_check);
		}

		generate_const_file!(self.memory.writeback_timeout);
	}
}
