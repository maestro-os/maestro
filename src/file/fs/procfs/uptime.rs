//! The uptime node returns the amount of time elapsed since the system started up.

use crate::errno::EResult;
use crate::errno::Errno;
use crate::file::fs::kernfs::content::KernFSContent;
use crate::file::fs::kernfs::node::KernFSNode;
use crate::file::FileContent;
use crate::file::Mode;
use crate::util::io::IO;
use core::cmp::min;

/// The uptime node.
#[derive(Debug)]
pub struct Uptime {}

impl KernFSNode for Uptime {
	fn get_mode(&self) -> Mode {
		0o444
	}

	fn get_content(&mut self) -> EResult<KernFSContent<'_>> {
		Ok(KernFSContent::Dynamic(FileContent::Regular))
	}
}

impl IO for Uptime {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&mut self, offset: u64, buff: &mut [u8]) -> Result<(u64, bool), Errno> {
		// TODO
		let content = crate::format!("0.00 0.00\n")?;
		let content_bytes = content.as_bytes();

		// Copy content to userspace buffer
		let len = min((content_bytes.len() as u64 - offset) as usize, buff.len());
		buff[..len].copy_from_slice(&content_bytes[(offset as usize)..(offset as usize + len)]);

		let eof = (offset + len as u64) >= content_bytes.len() as u64;
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
