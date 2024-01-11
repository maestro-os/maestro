//! This file implements the configuration file for compilation.

use serde::Deserialize;
use std::fs;
use std::io;

/// The path to the configuration file.
pub const PATH: &str = "config.toml";

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
		println!("cargo:rerun-if-changed={PATH}");
		let config_str = fs::read_to_string(PATH)?;
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
