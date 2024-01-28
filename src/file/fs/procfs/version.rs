//! The `/proc/version` file returns the version of the kernel.

use crate::errno::EResult;
use crate::errno::Errno;
use crate::file::fs::kernfs::content::KernFSContent;
use crate::file::fs::kernfs::node::KernFSNode;
use crate::file::FileContent;
use crate::util::io::IO;
use core::cmp::min;

/// Structure representing the version node.
#[derive(Debug)]
pub struct Version {}

impl KernFSNode for Version {
	fn get_content(&mut self) -> EResult<KernFSContent<'_>> {
		Ok(KernFSContent::Dynamic(FileContent::Regular))
	}
}

impl IO for Version {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&mut self, offset: u64, buff: &mut [u8]) -> Result<(u64, bool), Errno> {
		// TODO const format
		let version = crate::format!("{} version {}\n", crate::NAME, crate::VERSION)?;
		let version_bytes = version.as_bytes();

		// Copy content to userspace buffer
		let len = min((version_bytes.len() as u64 - offset) as usize, buff.len());
		buff[..len].copy_from_slice(&version_bytes[(offset as usize)..(offset as usize + len)]);

		let eof = (offset + len as u64) >= version_bytes.len() as u64;
		Ok((len as _, eof))
	}

	fn write(&mut self, _offset: u64, _buff: &[u8]) -> Result<u64, Errno> {
		Err(errno!(EINVAL))
	}

	fn poll(&mut self, _mask: u32) -> Result<u32, Errno> {
		// TODO
		todo!();
	}
}
