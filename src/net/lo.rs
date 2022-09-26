//! This module implements the local loopback.

use crate::errno::Errno;
use super::BindAddress;
use super::Interface;
use super::MAC;

/// Local loopback interfaces allows the system to write data to itself.
pub struct LocalLoopback {}

impl Interface for LocalLoopback {
	fn get_name(&self) -> &[u8] {
		b"lo"
	}

	fn is_up(&self) -> bool {
		true
	}

	fn get_mac(&self) -> &MAC {
		&[0xff; 6]
	}

	fn get_addresses(&self) -> &[BindAddress] {
		&[]
	}

	fn read(&mut self, _buff: &mut [u8]) -> Result<(u64, bool), Errno> {
		// TODO Write to ring buffer
		todo!();
	}

	fn write(&mut self, _buff: &[u8]) -> Result<u64, Errno> {
		// TODO Read from ring buffer
		todo!();
	}
}
