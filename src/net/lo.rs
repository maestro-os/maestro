//! This module implements the local loopback.

use super::Address;
use super::BindAddress;
use super::Interface;
use super::MAC;
use crate::errno::Errno;

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
		&[0x00; 6]
	}

	fn get_addresses(&self) -> &[BindAddress] {
		&[
			BindAddress {
				addr: Address::IPv4([127, 0, 0, 1]),
				subnet_mask: 8,
			},
			BindAddress {
				addr: Address::IPv6([
					0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
					0x00, 0x00, 0x01,
				]),
				subnet_mask: 128,
			},
		]
	}

	fn read(&mut self, _buff: &mut [u8]) -> Result<u64, Errno> {
		// TODO Write to ring buffer
		todo!();
	}

	fn write(&mut self, _buff: &[u8]) -> Result<u64, Errno> {
		// TODO Read from ring buffer
		todo!();
	}
}
