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

//! Network stack implementation.

pub mod buff;
pub mod icmp;
pub mod ip;
pub mod lo;
pub mod netlink;
pub mod osi;
pub mod sockaddr;
pub mod tcp;

use crate::{
	file::perm::{AccessProfile, ROOT_GID, ROOT_UID},
	net::sockaddr::{SockAddrIn, SockAddrIn6},
};
use buff::BuffList;
use core::{cmp::Ordering, mem::size_of};
use utils::{
	collections::{hashmap::HashMap, string::String, vec::Vec},
	errno,
	errno::{EResult, Errno},
	lock::Mutex,
	ptr::arc::Arc,
};

/// Type representing a Media Access Control (MAC) address.
pub type MAC = [u8; 6];

// TODO allow implementation of custom protocols

/// An enumeration of network address types.
#[derive(Debug, Eq, PartialEq)]
pub enum Address {
	/// Internet Protocol version 4.
	IPv4([u8; 4]),
	/// Internet Protocol version 6.
	IPv6([u8; 16]),
}

/// An address/subnet mask pair to be bound to an interface.
#[derive(Debug)]
pub struct BindAddress {
	/// The bound address.
	pub addr: Address,
	/// Subnet mask/prefix length.
	pub subnet_mask: u8,
}

impl BindAddress {
	/// Tells whether the bind address is suitable for transmission to the given destination
	/// address.
	pub fn is_matching(&self, addr: &Address) -> bool {
		fn check<const N: usize>(a: &[u8; N], b: &[u8; N], mask: usize) -> bool {
			a.array_chunks::<4>()
				.zip(b.array_chunks::<4>())
				.enumerate()
				.all(|(i, (a, b))| {
					let a = u32::from_ne_bytes(*a);
					let b = u32::from_ne_bytes(*b);

					let order = 32 - mask.saturating_sub(i * 32);
					let mask = !((1 << order) - 1);

					(a & mask) == (b & mask)
				})
		}

		match (&self.addr, addr) {
			(Address::IPv4(a), Address::IPv4(b)) => check(a, b, self.subnet_mask as _),
			(Address::IPv6(a), Address::IPv6(b)) => check(a, b, self.subnet_mask as _),
			_ => false,
		}
	}
}

/// Trait representing a network interface.
pub trait Interface {
	/// Returns the name of the interface.
	fn get_name(&self) -> &[u8];

	/// Tells whether the interface is UP.
	fn is_up(&self) -> bool;

	/// Returns the mac address of the interface.
	fn get_mac(&self) -> &MAC;

	/// Returns the list of addresses bound to the interface.
	fn get_addresses(&self) -> &[BindAddress];

	/// Reads data from the network interface and writes it into `buff`.
	///
	/// The function returns the number of bytes read.
	fn read(&mut self, buff: &mut [u8]) -> EResult<u64>;

	/// Reads data from `buff` and writes it into the network interface.
	///
	/// The function returns the number of bytes written.
	fn write(&mut self, buff: &BuffList<'_>) -> EResult<u64>;
}

/// An entry in the routing table.
pub struct Route {
	/// The destination address. If `None`, this is the default destination.
	dst: Option<BindAddress>,

	/// The name of the network interface.
	iface: String,
	/// The gateway's address.
	gateway: Address,

	/// The route's metric. The route with the lowest metric has priority.
	metric: u32,
}

impl Route {
	/// Tells whether the route matches the given address.
	pub fn is_matching(&self, addr: &Address) -> bool {
		// Check gateway
		if &self.gateway == addr {
			return true;
		}

		let Some(ref dst) = self.dst else {
			// Default route
			return true;
		};

		// Check with netmask
		dst.is_matching(addr)
	}

	/// Compares the current route with the given route `other`.
	///
	/// Ordering is done so that the best route is the greatest.
	pub fn cmp_for(&self, other: &Self, addr: &Address) -> Ordering {
		// Check gateway
		let self_match = addr == &self.gateway;
		let other_match = addr == &other.gateway;

		self_match
			.cmp(&other_match)
			.then_with(|| {
				// Check for matching network prefix

				let self_match = self
					.dst
					.as_ref()
					.map(|dst| dst.is_matching(addr))
					// Default address
					.unwrap_or(true);

				let other_match = other
					.dst
					.as_ref()
					.map(|dst| dst.is_matching(addr))
					// Default address
					.unwrap_or(true);

				self_match.cmp(&other_match)
			})
			.then_with(|| {
				// Check metric
				self.metric.cmp(&other.metric)
			})
	}
}

/// The list of network interfaces.
pub static INTERFACES: Mutex<HashMap<String, Arc<Mutex<dyn Interface>>>> =
	Mutex::new(HashMap::new());
/// The routing table.
pub static ROUTING_TABLE: Mutex<Vec<Route>> = Mutex::new(Vec::new());

/// Registers the given network interface.
///
/// Arguments:
/// - `name` is the name of the interface.
/// - `iface` is the interface to register.
pub fn register_iface<I: 'static + Interface>(name: String, iface: I) -> EResult<()> {
	let mut interfaces = INTERFACES.lock();

	let i = Arc::new(Mutex::new(iface))?;
	interfaces.insert(name, i)?;

	Ok(())
}

/// Unregisters the network interface with the given name.
pub fn unregister_iface(name: &[u8]) {
	let mut interfaces = INTERFACES.lock();
	interfaces.remove(name);
}

/// Returns the network interface with the given name.
///
/// If the interface doesn't exist, thhe function returns `None`.
pub fn get_iface(name: &[u8]) -> Option<Arc<Mutex<dyn Interface>>> {
	INTERFACES.lock().get(name).cloned()
}

/// Returns the network interface to be used to transmit a packet to the given destination address.
pub fn get_iface_for(addr: Address) -> Option<Arc<Mutex<dyn Interface>>> {
	let routing_table = ROUTING_TABLE.lock();
	let route = routing_table
		.iter()
		.filter(|route| route.is_matching(&addr))
		.max_by(|a, b| a.cmp_for(b, &addr))?;
	get_iface(&route.iface)
}

/// Enumeration of socket domains.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SocketDomain {
	/// Local communication.
	AfUnix,
	/// IPv4 Internet Protocols.
	AfInet,
	/// IPv6 Internet Protocols.
	AfInet6,
	/// Kernel user interface device.
	AfNetlink,
	/// Low level packet interface.
	AfPacket,
}

impl TryFrom<u32> for SocketDomain {
	type Error = Errno;

	fn try_from(id: u32) -> Result<Self, Self::Error> {
		match id {
			1 => Ok(Self::AfUnix),
			2 => Ok(Self::AfInet),
			10 => Ok(Self::AfInet6),
			16 => Ok(Self::AfNetlink),
			17 => Ok(Self::AfPacket),

			_ => Err(errno!(EAFNOSUPPORT)),
		}
	}
}

impl SocketDomain {
	/// Returns the associated ID.
	pub fn get_id(&self) -> u32 {
		match self {
			Self::AfUnix => 1,
			Self::AfInet => 2,
			Self::AfInet6 => 10,
			Self::AfNetlink => 16,
			Self::AfPacket => 17,
		}
	}

	/// Returns the size of the sockaddr structure for the domain.
	pub fn get_sockaddr_len(&self) -> usize {
		match self {
			Self::AfInet => size_of::<SockAddrIn>(),
			Self::AfInet6 => size_of::<SockAddrIn6>(),
			// TODO add others
			_ => 0,
		}
	}
}

impl AccessProfile {
	/// Tells whether the agent has the permission to use the socket domain.
	pub fn can_use_sock_domain(&self, domain: &SocketDomain) -> bool {
		match domain {
			SocketDomain::AfPacket => self.get_euid() == ROOT_UID || self.get_egid() == ROOT_GID,
			_ => true,
		}
	}
}

/// Enumeration of socket types.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum SocketType {
	/// Sequenced, reliable, two-way, connection-based byte streams.
	SockStream,
	/// Datagrams.
	SockDgram,
	/// Sequenced, reliable, two-way connection-based data transmission path for datagrams of
	/// fixed maximum length.
	SockSeqpacket,
	/// Raw network protocol access.
	SockRaw,
}

impl TryFrom<u32> for SocketType {
	type Error = Errno;

	fn try_from(id: u32) -> Result<Self, Self::Error> {
		match id {
			1 => Ok(Self::SockStream),
			2 => Ok(Self::SockDgram),
			5 => Ok(Self::SockSeqpacket),
			3 => Ok(Self::SockRaw),

			_ => Err(errno!(EPROTONOSUPPORT)),
		}
	}
}

impl SocketType {
	/// Returns the associated ID.
	pub fn get_id(&self) -> u32 {
		match self {
			Self::SockStream => 1,
			Self::SockDgram => 2,
			Self::SockSeqpacket => 5,
			Self::SockRaw => 3,
		}
	}

	/// Tells whether the socket type is using stream communications.
	pub fn is_stream(&self) -> bool {
		matches!(self, Self::SockStream | Self::SockSeqpacket)
	}
}

impl AccessProfile {
	/// Tells whether the agent has the permission to use the socket type.
	pub fn can_use_sock_type(&self, sock_type: &SocketType) -> bool {
		match sock_type {
			SocketType::SockRaw => self.get_euid() == ROOT_UID || self.get_egid() == ROOT_GID,
			_ => true,
		}
	}
}

/// Socket network stack descriptor.
#[derive(Debug)]
pub struct SocketDesc {
	/// The socket's domain.
	pub domain: SocketDomain,
	/// The socket's type.
	pub type_: SocketType,
	/// The socket's protocol. `0` means using the default protocol for the domain/type pair.
	pub protocol: i32,
}
