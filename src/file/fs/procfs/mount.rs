//! This module implements a procfs node which allows to get the list of mountpoint.

use crate::errno::Errno;
use crate::file::mountpoint;
use crate::util::IO;

/// Structure representing the mount node of the procfs.
pub struct ProcFSMountIO {}

impl IO for ProcFSMountIO {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&mut self, _offset: u64, buff: &mut [u8]) -> Result<u64, Errno> {
		let guard = mountpoint::MOUNT_POINTS.lock();
		let container = guard.get_mut();

		let iter = container.iter()
			.map(| (_, mp_mutex) | {
				let mp_guard = mp_mutex.lock();
				let mp = mp_guard.get();

				let source = "TODO"; // TODO
				let fs_type = "TODO"; // TODO
				let flags = "TODO"; // TODO

				crate::format!("{} {} {} {} 0 0", source, mp.get_path(), fs_type, flags)
			});

		// TODO Handle offset
		let mut i = 0;
		for mp in iter {
			if i >= buff.len() {
				break;
			}

			let remaining = buff.len() - i;
			buff[i..].copy_from_slice(&mp?.as_bytes()[remaining..]);

			i += remaining;
		}

		Ok(i as _)
	}

	fn write(&mut self, _offset: u64, _buff: &[u8]) -> Result<u64, Errno> {
		Err(errno!(EINVAL))
	}
}
