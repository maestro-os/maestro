//! This module implements the IP protocol.

use core::mem::size_of;
use core::slice;
use crate::crypto::checksum;
use crate::errno::Errno;
use super::BuffList;
use super::Layer;

/// The IPv4 header (RFC 791).
#[repr(C, packed)]
pub struct IPv4Header {
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

	/// TODO doc
	options: u32,
}

impl IPv4Header {
	/// Checks the checksum of the packet.
	///
	/// If correct, the function returns `true`.
	pub fn check_checksum(&self) -> bool {
		let slice = unsafe {
			slice::from_raw_parts(self as *const _ as *const u8, size_of::<Self>())
		};

		checksum::compute_rfc1071(slice) == 0
	}
}

/// The IPv6 header (RFC 8200).
#[repr(C, packed)]
pub struct IPv6Header {
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
pub struct IPv4Layer {}

impl Layer for IPv4Layer {
	fn transmit<'c, F>(
		&self,
		mut buff: BuffList<'c>,
		next: F
	) -> Result<(), Errno>
		where F: Fn(BuffList<'c>) -> Result<(), Errno> {
		let hdr = IPv4Header {
			version_ihl: 0, // TODO
			type_of_service: 0, // TODO
			total_length: 0, // TODO

			identification: 0, // TODO
			flags_fragment_offset: 0, // TODO

			ttl: 0, // TODO
			protocol: 0, // TODO
			hdr_checksum: 0, // TODO

			src_addr: [0; 4], // TODO
			dst_addr: [0; 4], // TODO

			options: 0, // TODO
		};
		let hdr_buff = unsafe {
			slice::from_raw_parts::<u8>(
				&hdr as *const _ as *const _,
				size_of::<IPv4Header>()
			)
		};

		buff.push_front(hdr_buff.into());
		next(buff)
	}
}

// TODO IPv6
