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

//! This module implements the IP protocol.

use super::{buff::BuffList, osi::Layer};
use crate::crypto::checksum;
use core::{mem::size_of, slice};
use utils::{boxed::Box, errno::EResult};

/// The default TTL value.
const DEFAULT_TTL: u8 = 128;

/// IPv4 flag: Do not fragment the packet
const FLAG_DF: u8 = 0b010;
/// IPv4 flag: More fragments are to come after this one
const FLAG_MF: u8 = 0b100;

/// Protocol: TCP
pub const PROTO_TCP: u8 = 0x06;
/// Protocol: UDP
pub const PROTO_UDP: u8 = 0x11;

/// The IPv4 header (RFC 791).
#[repr(C, packed)]
struct IPv4Header {
	/// The version of the header with the IHL (header length).
	version_ihl: u8,
	/// The type of service.
	type_of_service: u8,
	/// The total length of the datagram.
	total_length: u16,

	/// TODO doc
	identification: u16,
	/// TODO doc
	flags_fragment_offset: u16,

	/// Time-To-Live.
	ttl: u8,
	/// Protocol number.
	protocol: u8,
	/// The checksum of the header (RFC 1071).
	hdr_checksum: u16,

	/// Source address.
	src_addr: [u8; 4],
	/// Destination address.
	dst_addr: [u8; 4],
}

impl IPv4Header {
	/// Checks the checksum of the packet.
	///
	/// If correct, the function returns `true`.
	pub fn check_checksum(&self) -> bool {
		let slice =
			unsafe { slice::from_raw_parts(self as *const _ as *const u8, size_of::<Self>()) };

		checksum::compute_rfc1071(slice) == 0
	}

	/// Computes the checksum of the header and writes it into the appropriate field.
	pub fn compute_checksum(&mut self) {
		self.hdr_checksum = 0;

		let slice =
			unsafe { slice::from_raw_parts(self as *const _ as *const u8, size_of::<Self>()) };
		self.hdr_checksum = checksum::compute_rfc1071(slice);
	}
}

/// The IPv6 header (RFC 8200).
#[repr(C, packed)]
struct IPv6Header {
	/// The version, traffic class and flow label.
	version_traffic_class_flow_label: u32,

	/// The length of the payload.
	payload_length: u16,
	/// The type of the next header.
	next_header: u8,
	/// The number of hops remaining before discarding the packet.
	hop_limit: u8,

	/// Source address.
	src_addr: [u8; 16],
	/// Destination address.
	dst_addr: [u8; 16],
}

/// The network layer for the IPv4 protocol.
pub struct IPv4Layer {
	/// The protocol ID.
	pub protocol: u8,

	/// The destination IPv4.
	pub dst_addr: [u8; 4],
}

impl Layer for IPv4Layer {
	fn transmit<'c, F>(&self, mut buff: BuffList<'c>, next: F) -> EResult<()>
	where
		F: Fn(BuffList<'c>) -> EResult<()>,
	{
		let hdr_len = size_of::<IPv4Header>() as u16; // TODO add options support?

		let dscp = 0; // TODO
		let ecn = 0; // TODO

		// TODO check endianess
		let mut hdr = IPv4Header {
			version_ihl: 4 | (((hdr_len / 4) as u8) << 4),
			type_of_service: (dscp << 2) | ecn,
			total_length: hdr_len + buff.len() as u16,

			identification: 0,        // TODO
			flags_fragment_offset: 0, // TODO

			// TODO allow setting a different value
			ttl: DEFAULT_TTL,
			protocol: self.protocol,
			hdr_checksum: 0,

			src_addr: [0; 4], // IPADDR_ANY
			dst_addr: self.dst_addr,
		};
		hdr.compute_checksum();

		let hdr_buff = unsafe {
			slice::from_raw_parts::<u8>(&hdr as *const _ as *const _, size_of::<IPv4Header>())
		};

		buff.push_front(hdr_buff.into());
		next(buff)
	}
}

/// Builds an IPv4 layer with the given `sockaddr`.
pub fn inet_build(_sockaddr: &[u8]) -> EResult<Box<dyn Layer>> {
	// TODO
	todo!()
}

// TODO IPv6

/// Builds an IPv6 layer with the given `sockaddr`.
pub fn inet6_build(_sockaddr: &[u8]) -> EResult<Box<dyn Layer>> {
	// TODO
	todo!()
}
