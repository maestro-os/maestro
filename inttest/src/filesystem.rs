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

//! Filesystem testing.

use crate::{
	log, test_assert, test_assert_eq, util,
	util::{unprivileged, TestError, TestResult},
};
use std::{
	fs,
	fs::OpenOptions,
	io,
	io::{Read, Seek, SeekFrom, Write},
	os::{fd::AsRawFd, unix, unix::fs::MetadataExt},
	path::Path,
};

pub fn basic() -> TestResult {
	log!("File creation");
	let path = Path::new("test");
	let mut file = OpenOptions::new()
		.create_new(true)
		.read(true)
		.write(true)
		.open(path)?;

	log!("File write");
	let len = file.write(b"hello world!")?;
	test_assert_eq!(len, 12);

	log!("File seek");
	let off = file.seek(SeekFrom::Start(0))?;
	test_assert_eq!(off, 0);
	let off = file.seek(SeekFrom::End(0))?;
	test_assert_eq!(off, 12);

	log!("File read");
	let mut buf: [u8; 16] = [0; 16];
	let len = file.read(&mut buf)?;
	test_assert_eq!(len, 0);
	test_assert_eq!(&buf, &[0u8; 16]);
	let off = file.seek(SeekFrom::Start(0))?;
	test_assert_eq!(off, 0);
	let len = file.read(&mut buf)?;
	test_assert_eq!(len, 12);
	test_assert_eq!(&buf, b"hello world!\0\0\0\0");

	log!("File overwriting");
	let off = file.seek(SeekFrom::Start(6))?;
	test_assert_eq!(off, 6);
	let len = file.write(b"abcdefghij")?;
	test_assert_eq!(len, 10);

	log!("File chmod");
	for mode in 0..=0o7777 {
		util::fchmod(file.as_raw_fd(), mode)?;
		let stat = util::fstat(file.as_raw_fd())?;
		test_assert_eq!(stat.st_mode & 0o7777, mode);
	}

	// TODO change access/modification times

	log!("File remove");
	test_assert!(path.exists());
	fs::remove_file(path)?;
	test_assert!(!path.exists());
	test_assert!(matches!(fs::remove_file(path), Err(e) if e.kind() == io::ErrorKind::NotFound));

	log!("File remove defer");
	let off = file.seek(SeekFrom::End(0))?;
	test_assert_eq!(off, 16);
	let off = file.seek(SeekFrom::Start(0))?;
	test_assert_eq!(off, 0);
	let mut buf: [u8; 16] = [0; 16];
	let len = file.read(&mut buf)?;
	test_assert_eq!(len, 16);
	test_assert_eq!(&buf, b"hello abcdefghij");

	Ok(())
}

pub fn directories() -> TestResult {
	log!("Create directory at non-existent location (invalid)");
	let res = fs::create_dir("/abc/def");
	test_assert!(matches!(res, Err(e) if e.kind() == io::ErrorKind::NotFound));

	log!("Create directories");
	fs::create_dir_all("/abc/def/ghi")?;
	log!("Stat `/abc`");
	let stat = util::stat("/abc")?;
	log!("Stat `/abc/def`");
	test_assert_eq!(stat.st_nlink, 3);
	let stat = util::stat("/abc/def")?;
	test_assert_eq!(stat.st_nlink, 3);
	log!("Stat `/abc/def/ghi`");
	let stat = util::stat("/abc/def/ghi")?;
	test_assert_eq!(stat.st_nlink, 2);
	test_assert_eq!(stat.st_mode & 0o7777, 0o755);
	log!("Cleanup");
	fs::remove_dir_all("/abc/def")?;

	log!("Create entries");
	for i in 0..100 {
		fs::create_dir(format!("/abc/{i}"))?;
	}
	log!("List entries");
	let mut entries = fs::read_dir("/abc")?
		.map(|ent| {
			let ent = ent?;
			test_assert!(ent.file_type()?.is_dir());
			let file_name = ent.file_name();
			let file_name = file_name
				.to_str()
				.ok_or_else(|| TestError("invalid entry".to_owned()))?;
			Ok(file_name.parse::<u32>()? as _)
		})
		.collect::<Result<Vec<u32>, TestError>>()?;
	entries.sort_unstable();
	for (a, b) in entries.into_iter().enumerate() {
		test_assert_eq!(a as u32, b);
	}

	log!("Remove non-empty directory (invalid)");
	let res = fs::remove_dir("/abc");
	test_assert!(matches!(res, Err(e) if e.kind() == io::ErrorKind::DirectoryNotEmpty));

	log!("Cleanup");
	fs::remove_dir_all("/abc")?;
	Ok(())
}

pub fn dir_perms() -> TestResult {
	fs::create_dir_all("/foo/bar")?;
	util::chown("/foo", 1000, 1000)?;
	util::chown("/foo/bar", 1000, 1000)?;

	unprivileged(|| {
		log!("No permission");
		util::chmod("/foo", 0o000)?;
		util::stat("/foo")?;
		let res = util::stat("/foo/bar");
		test_assert!(matches!(res, Err(e) if e.kind() == io::ErrorKind::PermissionDenied));
		let res = fs::read_dir("/foo");
		test_assert!(matches!(res, Err(e) if e.kind() == io::ErrorKind::PermissionDenied));

		log!("Entries list and write without search permissions");
		for mode in [0o444, 0o666] {
			util::chmod("/foo", mode)?;
			let res = util::stat("/foo/bar");
			test_assert!(matches!(res, Err(e) if e.kind() == io::ErrorKind::PermissionDenied));
			fs::read_dir("/foo")?;
			let res = fs::create_dir("/foo/no_perm");
			test_assert!(matches!(res, Err(e) if e.kind() == io::ErrorKind::PermissionDenied));
		}

		log!("Search permissions");
		util::chmod("/foo", 0o555)?;
		fs::read_dir("/foo")?;
		let res = fs::create_dir("/foo/no_perm");
		test_assert!(matches!(res, Err(e) if e.kind() == io::ErrorKind::PermissionDenied));

		Ok(())
	})??;

	log!("Cleanup");
	fs::remove_dir_all("/foo")?;
	Ok(())
}

pub fn hardlinks() -> TestResult {
	log!("Create link to directory (invalid)");
	fs::create_dir("test_dir")?;
	let res = fs::hard_link("test_dir", "bad_link");
	test_assert!(matches!(res, Err(e) if e.kind() == io::ErrorKind::PermissionDenied));
	// Check the link has not been created
	let res = fs::remove_dir("bad_link");
	test_assert!(matches!(res, Err(e) if e.kind() == io::ErrorKind::NotFound));
	log!("Cleanup");
	fs::remove_dir("test_dir")?;

	log!("Create link to file");
	fs::hard_link("maestro-test", "good_link")?;
	let inode0 = util::stat("maestro-test")?.st_ino;
	let inode1 = util::stat("good_link")?.st_ino;
	test_assert_eq!(inode0, inode1);
	log!("Remove link to file");
	fs::remove_file("good_link")?;
	util::stat("maestro-test")?;
	let res = util::stat("good_link");
	test_assert!(matches!(res, Err(e) if e.kind() == io::ErrorKind::NotFound));

	log!("Create link to file that don't exist (invalid)");
	let res = fs::hard_link("not_found", "link");
	test_assert!(matches!(res, Err(e) if e.kind() == io::ErrorKind::NotFound));

	Ok(())
}

pub fn symlinks() -> TestResult {
	log!("Create link");
	unix::fs::symlink("maestro-test", "testlink")?;
	log!("Cleanup");
	fs::remove_file("testlink")?;

	log!("Create directory");
	fs::create_dir("target")?;
	log!("Create link to directory");
	unix::fs::symlink("target", "link")?;
	log!("Stat link");
	test_assert!(fs::symlink_metadata("link")?.is_symlink());
	log!("Stat directory");
	test_assert!(fs::symlink_metadata("link/")?.is_dir());

	log!("Make dangling");
	fs::remove_dir("target")?;
	log!("Stat link");
	test_assert!(fs::symlink_metadata("link")?.is_symlink());
	log!("Stat directory");
	test_assert!(
		matches!(fs::symlink_metadata("link/"), Err(e) if e.kind() == io::ErrorKind::NotFound)
	);
	log!("Cleanup");
	fs::remove_file("link")?;

	Ok(())
}

pub fn rename() -> TestResult {
	log!("Create file");
	{
		let mut file = OpenOptions::new()
			.create_new(true)
			.read(true)
			.write(true)
			.open("old")?;
		file.write_all(b"abcdef")?;
	}

	log!("Rename");
	fs::rename("old", "new")?;
	log!("Stat old file");
	test_assert!(matches!(fs::metadata("old"), Err(e) if e.kind() == io::ErrorKind::NotFound));
	log!("Stat new file");
	let metadata = fs::metadata("new")?;
	test_assert!(metadata.is_file());
	test_assert_eq!(metadata.len(), 6);
	test_assert_eq!(metadata.nlink(), 1);
	log!("Read new file");
	test_assert_eq!(fs::read("new")?, b"abcdef");
	log!("Cleanup");
	fs::remove_file("new")?;

	log!("Create directories");
	fs::create_dir_all("old/foo/bar")?;
	log!("Rename");
	fs::rename("old", "new")?;
	log!("Stat old directory");
	test_assert!(matches!(fs::metadata("old"), Err(e) if e.kind() == io::ErrorKind::NotFound));
	log!("Stat new directories");
	for (path, links) in [("new", 3), ("foo", 3), ("bar", 2)] {
		let metadata = fs::metadata(path)?;
		test_assert!(metadata.is_dir());
		test_assert_eq!(metadata.nlink(), links);
	}
	log!("Cleanup");
	fs::remove_dir_all("new")?;
	test_assert!(matches!(fs::metadata("new"), Err(e) if e.kind() == io::ErrorKind::NotFound));

	// TODO test moving across mountpoints

	Ok(())
}

pub fn fifo() -> TestResult {
	log!("Create fifo");
	util::mkfifo("fifo", 0o666)?;
	// TODO test read/write (need another thread/process)
	log!("Cleanup");
	fs::remove_file("fifo")?;

	// TODO test on tmpfs

	Ok(())
}
