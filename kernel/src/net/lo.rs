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

//! This module implements the local loopback.

use super::{buff::BuffList, Address, BindAddress, Interface, MAC};
use utils::errno::EResult;

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

	fn read(&mut self, _buff: &mut [u8]) -> EResult<u64> {
		// TODO Write to ring buffer
		todo!();
	}

	fn write(&mut self, _buff: &BuffList<'_>) -> EResult<u64> {
		// TODO Read from ring buffer
		todo!();
	}
}
