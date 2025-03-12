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
use memmap2::MmapOptions;
use std::{
	fs,
	fs::OpenOptions,
	io,
	io::{Read, Seek, SeekFrom, Write},
	os::{fd::AsRawFd, unix, unix::fs::MetadataExt},
	path::Path,
};

pub fn basic(root: &Path) -> TestResult {
	log!("File creation");
	let path = root.join("test");
	let mut file = OpenOptions::new()
		.create_new(true)
		.read(true)
		.write(true)
		.open(&path)?;

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
	fs::remove_file(&path)?;
	test_assert!(!path.exists());
	test_assert!(matches!(fs::remove_file(&path), Err(e) if e.kind() == io::ErrorKind::NotFound));

	log!("File use after remove");
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

// TODO O_APPEND

pub fn mmap(root: &Path) -> TestResult {
	log!("Create file");
	let path = root.join("file");
	let mut file = OpenOptions::new()
		.create(true)
		.truncate(true)
		.read(true)
		.write(true)
		.open(&path)?;

	log!("Map a page");
	let mut mmap = unsafe { MmapOptions::new().offset(4096).len(4096).map_mut(&file)? };
	test_assert!(mmap.iter().all(|b| *b == 0));

	log!("Write on page");
	mmap.fill(1);

	log!("Read from file");
	let content = fs::read(&path)?;
	test_assert_eq!(content.len(), 8192);
	test_assert!(content.iter().enumerate().all(|(i, b)| if i < 4096 {
		*b == 0
	} else {
		*b == 1
	}));

	log!("Write to file");
	file.seek(SeekFrom::Start(0))?;
	let content: Vec<u8> = (0..8192).map(|_| 2).collect();
	file.write_all(&content)?;

	log!("Remove file");
	fs::remove_file(&path)?;
	test_assert!(!path.exists());

	log!("Check the file's content is still mapped");
	test_assert!(mmap.iter().all(|b| *b == 2));

	Ok(())
}

pub fn directories(root: &Path) -> TestResult {
	log!("Create directory at non-existent location (invalid)");
	let path = root.join("abc/def");
	let res = fs::create_dir(&path);
	test_assert!(matches!(res, Err(e) if e.kind() == io::ErrorKind::NotFound));

	log!("Create directories");
	fs::create_dir_all(root.join("abc/def/ghi"))?;
	let mut path = root.to_path_buf();
	for (dir, links) in [("abc", 3), ("def", 3), ("ghi", 2)] {
		path = path.join(dir);
		log!("Stat `{}`", path.display());
		let stat = util::stat(&path)?;
		test_assert_eq!(stat.st_mode & 0o7777, 0o755);
		test_assert_eq!(stat.st_nlink, links);
	}
	log!("Cleanup");
	fs::remove_dir_all(root.join("abc/def/ghi"))?;

	log!("Create entries");
	for i in 0..100 {
		fs::create_dir(root.join(format!("abc/{i}")))?;
	}
	log!("List entries");
	let path = root.join("abc");
	let mut entries = fs::read_dir(&path)?
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
	let res = fs::remove_dir(&path);
	test_assert!(matches!(res, Err(e) if e.kind() == io::ErrorKind::DirectoryNotEmpty));

	log!("Cleanup");
	fs::remove_dir_all(path)?;
	Ok(())
}

pub fn dir_perms(root: &Path) -> TestResult {
	let dir_foo = root.join("foo");
	let dir_bar = dir_foo.join("bar");
	let dir_no_perm = dir_foo.join("no_perm");
	fs::create_dir_all(&dir_bar)?;
	util::chown(&dir_foo, 1000, 1000)?;
	util::chown(&dir_bar, 1000, 1000)?;

	unprivileged(|| {
		log!("No permission");
		util::chmod(&dir_foo, 0o000)?;
		util::stat(&dir_foo)?;
		let res = util::stat(&dir_bar);
		test_assert!(matches!(res, Err(e) if e.kind() == io::ErrorKind::PermissionDenied));
		let res = fs::read_dir(&dir_foo);
		test_assert!(matches!(res, Err(e) if e.kind() == io::ErrorKind::PermissionDenied));

		log!("Entries list and write without search permissions");
		for mode in [0o444, 0o666] {
			util::chmod(&dir_foo, mode)?;
			let res = util::stat(&dir_bar);
			test_assert!(matches!(res, Err(e) if e.kind() == io::ErrorKind::PermissionDenied));
			fs::read_dir(&dir_foo)?;
			let res = fs::create_dir(&dir_no_perm);
			test_assert!(matches!(res, Err(e) if e.kind() == io::ErrorKind::PermissionDenied));
		}

		log!("Search permissions");
		util::chmod(&dir_foo, 0o555)?;
		fs::read_dir(&dir_foo)?;
		let res = fs::create_dir(&dir_no_perm);
		test_assert!(matches!(res, Err(e) if e.kind() == io::ErrorKind::PermissionDenied));

		Ok(())
	})??;

	log!("Cleanup");
	fs::remove_dir_all(&dir_foo)?;
	Ok(())
}

pub fn hardlinks(root: &Path) -> TestResult {
	let test_dir = root.join("test_dir");
	let file = root.join("file");
	let link = root.join("link");

	log!("Create link to directory (invalid)");
	fs::create_dir(&test_dir)?;
	let res = fs::hard_link(&test_dir, &link);
	test_assert!(matches!(res, Err(e) if e.kind() == io::ErrorKind::PermissionDenied));
	// Check the link has not been created
	let res = fs::remove_dir(&link);
	test_assert!(matches!(res, Err(e) if e.kind() == io::ErrorKind::NotFound));
	log!("Cleanup");
	fs::remove_dir(&test_dir)?;

	log!("Create link to file");
	fs::write(&file, b"abc")?;
	fs::hard_link(&file, &link)?;
	log!("Stat original");
	let inode0 = util::stat(&file)?.st_ino;
	log!("Stat link");
	let inode1 = util::stat(&link)?.st_ino;
	test_assert_eq!(inode0, inode1);
	log!("Remove link to file");
	fs::remove_file(&link)?;
	util::stat(&file)?;
	let res = util::stat(&link);
	test_assert!(matches!(res, Err(e) if e.kind() == io::ErrorKind::NotFound));
	log!("Cleanup");
	fs::remove_file(&file)?;

	log!("Create link to file that does not exist (invalid)");
	let res = fs::hard_link(&file, &link);
	test_assert!(matches!(res, Err(e) if e.kind() == io::ErrorKind::NotFound));

	Ok(())
}

pub fn symlinks(root: &Path) -> TestResult {
	let target = root.join("target");
	let link = root.join("link");
	let link_slash = root.join("link/");

	log!("Create link");
	fs::write(&target, b"abc")?;
	unix::fs::symlink(&target, &link)?;
	log!("Cleanup");
	fs::remove_file(&target)?;
	fs::remove_file(&link)?;

	log!("Create directory");
	fs::create_dir(&target)?;
	log!("Create link to directory");
	unix::fs::symlink(&target, &link)?;
	log!("Stat link");
	test_assert!(fs::symlink_metadata(&link)?.is_symlink());
	log!("Stat directory");
	test_assert!(fs::symlink_metadata(&link_slash)?.is_dir());

	log!("Make dangling");
	fs::remove_dir(&target)?;
	log!("Stat link");
	test_assert!(fs::symlink_metadata(&link)?.is_symlink());
	log!("Stat directory");
	test_assert!(
		matches!(fs::symlink_metadata(&link_slash), Err(e) if e.kind() == io::ErrorKind::NotFound)
	);
	log!("Cleanup");
	fs::remove_file(&link)?;

	Ok(())
}

pub fn rename(root: &Path) -> TestResult {
	let old = root.join("old");
	let new = root.join("new");

	log!("Create file");
	fs::write(&old, b"abcdef")?;

	log!("Rename");
	fs::rename(&old, &new)?;
	log!("Stat old file");
	test_assert!(matches!(fs::metadata(&old), Err(e) if e.kind() == io::ErrorKind::NotFound));
	log!("Stat new file");
	let metadata = fs::metadata(&new)?;
	test_assert!(metadata.is_file());
	test_assert_eq!(metadata.len(), 6);
	test_assert_eq!(metadata.nlink(), 1);
	log!("Read new file");
	test_assert_eq!(fs::read(&new)?, b"abcdef");
	log!("Cleanup");
	fs::remove_file(&new)?;

	// FIXME: moving a directory is broken
	log!("Create directories");
	fs::create_dir_all(old.join("foo/bar"))?;
	log!("Rename");
	fs::rename(&old, &new)?;
	log!("Stat old directory");
	test_assert!(matches!(fs::metadata(&old), Err(e) if e.kind() == io::ErrorKind::NotFound));
	log!("Stat new directories");
	let mut path = root.to_path_buf();
	for (dir, links) in [("new", 3), ("foo", 3), ("bar", 2)] {
		path = path.join(dir);
		let metadata = fs::metadata(&path)?;
		test_assert!(metadata.is_dir());
		test_assert_eq!(metadata.nlink(), links);
	}
	log!("Cleanup");
	fs::remove_dir_all(&new)?;
	test_assert!(matches!(fs::metadata(&new), Err(e) if e.kind() == io::ErrorKind::NotFound));

	// TODO test moving across mountpoints

	Ok(())
}

pub fn fifo(root: &Path) -> TestResult {
	log!("Create fifo");
	let path = root.join("fifo");
	util::mkfifo(&path, 0o666)?;

	// TODO test read/write (need another thread/process)

	log!("Cleanup");
	fs::remove_file(path)?;

	Ok(())
}
