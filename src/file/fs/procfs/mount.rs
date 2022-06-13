//! This module implements a procfs node which allows to get the list of mountpoint.

use crate::errno::Errno;
use crate::util::IO;

/// Structure representing the mount node of the procfs.
pub struct ProcFSMountIO {}

impl IO for ProcFSMountIO {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&mut self, _offset: u64, _buff: &mut [u8]) -> Result<u64, Errno> {
		// TODO
		todo!();
	}

	fn write(&mut self, _offset: u64, _buff: &[u8]) -> Result<u64, Errno> {
		Err(errno!(EINVAL))
	}
}
