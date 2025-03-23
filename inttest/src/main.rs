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

//! Integration tests for Maestro.

#![feature(io_error_more)]

use crate::{
	mount::{mount, umount},
	util::TestResult,
};
use std::{path::Path, process::exit};

mod filesystem;
mod mount;
mod procfs;
mod signal;
mod util;

/*
 * TODO when the serial port is unlinked from the TTY,
 * setup the output so that it is printed on both the stdout and serial port
 */

struct TestSuite {
	name: &'static str,
	desc: &'static str,
	tests: &'static [Test],
}

struct Test {
	name: &'static str,
	desc: &'static str,
	start: fn() -> TestResult,
}

macro_rules! fs_suite {
	($root:literal) => {
		TestSuite {
			name: "filesystem",
			desc: concat!("Files and filesystem handling (", $root, ")"),
			tests: &[
				Test {
					name: "persistence",
					desc: "Leave a file which will be accessed from the outside to check writeback to disk works",
					start: || filesystem::persistence(Path::new($root)),
				},
				Test {
					name: "basic",
					desc: "Create, remove and modify the properties of a single file",
					start: || filesystem::basic(Path::new($root)),
				},
				Test {
					name: "mmap",
					desc: "Map a file",
					start: || filesystem::mmap(Path::new($root)),
				},
				// TODO private mapped file
				// TODO umask
				Test {
					name: "directories",
					desc: "Create, remove and modify the properties directories",
					start: || filesystem::directories(Path::new($root)),
				},
				Test {
					name: "dir_perms",
					desc: "Test directory permissions",
					start: || filesystem::dir_perms(Path::new($root)),
				},
				Test {
					name: "hardlinks",
					desc: "Test hard links",
					start: || filesystem::hardlinks(Path::new($root)),
				},
				Test {
					name: "symlinks",
					desc: "Test symbolic links",
					start: || filesystem::symlinks(Path::new($root)),
				},
				// TODO test with a lot of files
				// TODO test with big files
				// TODO try to fill the filesystem
				// FIXME
				/*Test {
					name: "rename",
					desc: "Test renaming files",
					start: || filesystem::rename(Path::new($root)),
				},*/
				Test {
					name: "fifo",
					desc: "Test FIFO files",
					start: || filesystem::fifo(Path::new($root)),
				},
				// TODO file socket
				// TODO check /dev/* contents
			],
		}
	};
}

/// The list of tests to perform.
const TESTS: &[TestSuite] = &[
	// TODO test partitions (both MBR and GPT)
	TestSuite {
		name: "mount",
		desc: "Filesystem mount",
		tests: &[
			Test {
				name: "procfs",
				desc: "Mount procfs",
				start: || mount("procfs", "/proc", "procfs"),
			},
			Test {
				name: "tmpfs",
				desc: "Mount tmpfs",
				start: || mount("tmpfs", "/tmp", "tmpfs"),
			},
			// TODO other filesystem types
		],
	},
	// TODO fork/clone (threads)
	// TODO anonymous map (both shared and private)
	fs_suite!("/"),
	fs_suite!("/tmp"),
	TestSuite {
		name: "signal",
		desc: "Test signals",
		tests: &[
			Test {
				name: "handler",
				desc: "Register and use a signal handler",
				start: signal::handler,
			}, /* TODO signal masking
			    * TODO pause */
		],
	},
	// TODO ELF files (execve)
	// TODO user/group file accesses (including SUID/SGID)
	// TODO time ((non-)monotonic clock, sleep and timer_*)
	// TODO termcaps
	// TODO SSE/MMX/AVX states consistency
	TestSuite {
		name: "procfs",
		desc: "Test correctness of the procfs filesystem",
		tests: &[
			Test {
				name: "/proc/self/cwd",
				desc: "/proc/self/cwd",
				start: procfs::cwd,
			},
			Test {
				name: "/proc/self/exe",
				desc: "/proc/self/exe",
				start: procfs::exe,
			},
			Test {
				name: "/proc/self/cmdline",
				desc: "/proc/self/cmdline",
				start: procfs::cmdline,
			},
			Test {
				name: "/proc/self/environ",
				desc: "/proc/self/environ",
				start: procfs::environ,
			},
			// TODO /proc/self/stat
		],
	},
	// TODO install required commands
	/*TestSuite {
		name: "command",
		desc: "Basic commands testing",
		tests: &[
			Test {
				name: "ls -l /",
				desc: "ls -l /",
				start: || exec(Command::new("ls").args(["-l", "/"])),
			},
			Test {
				name: "ls -lR /",
				desc: "ls -lR /",
				start: || exec(Command::new("ls").args(["-lR", "/"])),
			},
			// TODO `cat`
			// TODO `cat -e`
			// TODO `cp`
			// TODO `rm`
		],
	},*/
	// TODO scripts (Shell/Perl)
	// TODO compilation (C/C++/Rust)
	// TODO network
	TestSuite {
		name: "Unmount",
		desc: "Unmount filesystems",
		tests: &[
			Test {
				name: "procfs",
				desc: "Unmount procfs",
				start: || umount("/proc"),
			},
			Test {
				name: "tmpfs",
				desc: "Unmount tmpfs",
				start: || umount("/tmp"),
			},
		],
	},
];

fn main() {
	// The total number of tests
	let total: usize = TESTS.iter().map(|t| t.tests.len()).sum();
	// Start marker
	println!();
	println!("[START]");
	let mut success = 0;
	for suite in TESTS {
		println!("[SUITE] {}", suite.name);
		println!("[DESC] {}", suite.desc);
		for test in suite.tests {
			println!("[TEST] {}", test.name);
			println!("[DESC] {}", test.desc);
			let res = (test.start)();
			match res {
				Ok(_) => {
					success += 1;
					println!("[OK]")
				}
				Err(err) => println!("[KO] {}", err.0),
			}
		}
	}
	println!("[SUCCESS] {success}/{total}");
	// End marker
	println!("[END]");
	if success < total {
		exit(1);
	}
}
