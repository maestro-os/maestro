//! TODO doc

use serde::Deserialize;
use std::env;
use std::fs;
use std::io;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;
use std::process::exit;

/// The path to the configuration file.
const CONFIG_PATH: &str = "config.toml";

/// The debug section of the configuration file.
#[derive(Deserialize)]
struct ConfigDebug {
	/// Tells whether the kernel is compiled in debug mode.
	debug: bool,
	/// If enabled, selftesting is enabled.
	///
	/// This option requires debug mode to be enabled.
	test: bool,
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
struct Config {
	/// The CPU architecture for which the kernel is compiled.
	arch: String,

	/// Debug section.
	debug: ConfigDebug,
}

impl Config {
	/// Sets the crate's cfg flags according to the configuration.
	fn set_cfg(&self) {
		println!("cargo:rustc-cfg=config_arch=\"{}\"", self.arch);

		println!(
			"cargo:rustc-cfg=config_debug_debug=\"{}\"",
			self.debug.debug
		);
		if self.debug.debug {
			println!("cargo:rustc-cfg=config_debug_test=\"{}\"", self.debug.test);
		}
		println!(
			"cargo:rustc-cfg=config_debug_storage_test=\"{}\"",
			self.debug.storage_test
		);

		println!("cargo:rustc-cfg=config_debug_qemu=\"{}\"", self.debug.qemu);

		println!(
			"cargo:rustc-cfg=config_debug_malloc_magic=\"{}\"",
			self.debug.malloc_magic
		);
		println!(
			"cargo:rustc-cfg=config_debug_malloc_check=\"{}\"",
			self.debug.malloc_check
		);
	}
}

/// Lists paths to C and assembly files.
fn list_c_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
	let mut paths = vec![];

	for e in fs::read_dir(dir)? {
		let e = e?;
		let e_path = e.path();
		let e_type = e.file_type()?;

		if e_type.is_file() {
			let ext = e_path.extension().map(|s| s.to_str()).flatten();
			let keep = match ext {
				Some("c" | "s") => true,
				_ => false,
			};
			if !keep {
				continue;
			}

			paths.push(e_path);
		} else if e_type.is_dir() {
			list_c_files(&e_path)?;
		}
	}

	Ok(paths)
}

/// Returns the triplet for the given architecture.
///
/// If the architecture is not supported, the function returns `None`.
fn arch_to_triplet(arch: &str) -> io::Result<String> {
	let path = PathBuf::from(format!("arch/{arch}/triplet"));
	let content = fs::read_to_string(path)?;

	Ok(content.trim().into())
}

/// Compiles the vDSO.
///
/// `arch` is the architecture to compile for.
fn compile_vdso(arch: &str) -> io::Result<()> {
	let triplet = arch_to_triplet(arch)?;

	cc::Build::new()
		.flag("-nostdlib")
		.flag("-ffreestanding")
		.flag("-mno-red-zone")
		.flag("-Wall")
		.flag("-Wextra")
		.flag("-Werror")
		.pic(true)
		.flag("-Tvdso/linker.ld")
		.shared_flag(true)
		.target(&triplet)
		.file(format!("vdso/{arch}.s"))
		.compile("vdso.so");

	println!("cargo:rerun-if-changed=vdso/linker.ld");
	println!("cargo:rerun-if-changed=vdso/{arch}.s");

	Ok(())
}

/// Compiles the C and assembly code.
///
/// `arch` is the architecture to compile for.
fn compile_c(arch: &str) -> io::Result<()> {
	let triplet = arch_to_triplet(arch)?;

	let debug = env::var("PROFILE").unwrap() == "debug";
	let opt_level: u32 = env::var("OPT_LEVEL").unwrap().parse().unwrap();

	let files = list_c_files(Path::new("src"))?;
	for f in &files {
		println!("cargo:rerun-if-changed={}", f.display());
	}

	cc::Build::new()
		.flag("-nostdlib")
		.flag("-ffreestanding")
		.flag("-fno-stack-protector")
		.flag("-fno-pic")
		.flag("-mno-red-zone")
		.flag("-Wall")
		.flag("-Wextra")
		.flag("-Werror")
		.flag(&format!("-Tarch/{}/linker.ld", arch))
		.target(&triplet)
		.debug(debug)
		.opt_level(opt_level)
		.files(files)
		.compile("libmaestro.a");

	Ok(())
}

/// Links the kernel library into an executable.
fn link_library() {
	println!("cargo:rustc-link-search=native=./");
	println!("cargo:rustc-link-lib=static=maestro");
	println!("cargo:rerun-if-changed=libmaestro.a");
}

fn main() {
	// Read compilation configuration
	let config_str = match fs::read_to_string(CONFIG_PATH) {
		Ok(content) => content,

		Err(e) if e.kind() == ErrorKind::NotFound => {
			eprintln!("Configuration file not found");
			eprintln!();
			eprintln!(
				"Please make sure the configuration file at `{}` exists`",
				CONFIG_PATH
			);
			eprintln!("An example configuration file can be found in `default.config.toml`");
			exit(1);
		}

		Err(e) => {
			eprintln!("Failed to read configuration file: {}", e);
			exit(1);
		}
	};
	let config: Config = toml::from_str(&config_str).unwrap_or_else(|e| {
		eprintln!("Failed to read configuration file: {}", e);
		exit(1);
	});

	config.set_cfg();

	compile_vdso(&config.arch).unwrap_or_else(|e| {
		eprintln!("vDSO compilation failed: {}", e);
		exit(1);
	});
	compile_c(&config.arch).unwrap_or_else(|e| {
		eprintln!("Compilation failed: {}", e);
		exit(1);
	});
	link_library();

	// Add the linker script
	println!("cargo:rerun-if-changed=arch/{}/linker.ld", config.arch);
	println!("cargo:rustc-link-arg=-Tarch/{}/linker.ld", config.arch);
}
