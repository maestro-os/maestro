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

//! This module implements the Internet Control Message Protocol.
//!
//! This procotol is defined by the following RFCs:
//! - With IPv4: RFC 792
//! - With IPv6 (ICMPv6): RFC 4443

/// An enumeration of ICMP packet types.
pub enum ICMPType {
	/// Used by ping to reply to an echo request.
	EchoReply,
	/// TODO doc
	DestinationUnreachable,
	/// TODO doc
	SourceQuench,
	/// TODO doc
	RedirectMessage,
	/// Used by ping to request an echo.
	EchoRequest,
	/// TODO doc
	RouterAdvertisement,
	/// TODO doc
	RouterSolicitation,
	/// TODO doc
	TimeExceeded,
	/// TODO doc
	ParameterProblem,
	/// TODO doc
	Timestamp,
	/// TODO doc
	TimestampReply,
	/// TODO doc
	InformationRequest,
	/// TODO doc
	InformationReply,
	/// TODO doc
	AddressMaskRequest,
	/// TODO doc
	AddressMaskReply,
	/// TODO doc
	Traceroute,
	/// TODO doc
	ExtendedEchoRequest,
	/// TODO doc
	ExtendedEchoReply,
}

impl ICMPType {
	/// Returns a type from its ID.
	///
	/// If no type match, the function returns None.
	pub fn from_type(id: u8) -> Option<Self> {
		match id {
			0 => Some(Self::EchoReply),
			3 => Some(Self::DestinationUnreachable),
			4 => Some(Self::SourceQuench),
			5 => Some(Self::RedirectMessage),
			8 => Some(Self::EchoRequest),
			9 => Some(Self::RouterAdvertisement),
			10 => Some(Self::RouterSolicitation),
			11 => Some(Self::TimeExceeded),
			12 => Some(Self::ParameterProblem),
			13 => Some(Self::Timestamp),
			14 => Some(Self::TimestampReply),
			15 => Some(Self::InformationRequest),
			16 => Some(Self::InformationReply),
			17 => Some(Self::AddressMaskRequest),
			18 => Some(Self::AddressMaskReply),
			30 => Some(Self::Traceroute),
			42 => Some(Self::ExtendedEchoRequest),
			43 => Some(Self::ExtendedEchoReply),

			_ => None,
		}
	}
}
