/*
 * Copyright 2026 Luc Lenôtre
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

use crate::{
	log, test_assert,
	util::{TestResult, exec},
};
use libc::{
	AF_NETLINK, AF_UNSPEC, ARPHRD_LOOPBACK, IFF_UP, IFLA_IFNAME, NETLINK_ROUTE, NLM_F_ACK,
	NLM_F_REQUEST, NLMSG_ERROR, RTM_NEWLINK, SOCK_DGRAM, c_int, c_uint, nlmsgerr, nlmsghdr, recv,
	sendto, sockaddr_nl, socket,
};
use std::{
	io,
	mem::{self, MaybeUninit},
	os::{
		fd::{AsRawFd, FromRawFd, OwnedFd},
		raw::{c_uchar, c_ushort},
	},
	process::Command,
	time::Duration,
};

#[repr(C)]
struct NlUpLinkMsg {
	hdr: nlmsghdr,
	ifinfo: ifinfomsg,

	attr_ifname: rtattr,
	attr_ifname_body: [u8; 3], // Hardcoded length for ease of use
}

#[repr(C)]
struct NlResponse {
	hdr: nlmsghdr,
	err: nlmsgerr,
}

#[repr(C)]
struct ifinfomsg {
	ifi_family: c_uchar,
	ifi_type: c_ushort,
	ifi_index: c_int,
	ifi_flags: c_uint,
	ifi_change: c_uint,
}

#[repr(C)]
struct rtattr {
	rta_len: c_ushort,
	rta_type: c_ushort,
}

pub fn setup() -> TestResult {
	log!("Open netlink socket");
	let sock = unsafe { socket(AF_NETLINK, SOCK_DGRAM, NETLINK_ROUTE) };
	if sock < 0 {
		return Err(io::Error::last_os_error().into());
	}
	let sock = unsafe { OwnedFd::from_raw_fd(sock) };

	log!("Up the local loopback");
	let msg = NlUpLinkMsg {
		hdr: nlmsghdr {
			nlmsg_len: size_of::<NlUpLinkMsg>() as _,
			nlmsg_type: RTM_NEWLINK,
			nlmsg_flags: (NLM_F_REQUEST | NLM_F_ACK) as _,
			nlmsg_seq: 1,
			nlmsg_pid: 0,
		},
		ifinfo: ifinfomsg {
			ifi_family: AF_UNSPEC as _,
			ifi_type: ARPHRD_LOOPBACK,
			ifi_index: 0,
			ifi_flags: IFF_UP as _,
			ifi_change: IFF_UP as _,
		},

		attr_ifname: rtattr {
			rta_len: size_of::<rtattr>() as c_ushort + 3,
			rta_type: IFLA_IFNAME,
		},
		attr_ifname_body: [b'l', b'o', 0],
	};
	let mut addr: sockaddr_nl = unsafe { mem::zeroed() };
	addr.nl_family = AF_NETLINK as _;
	// Send
	let res = unsafe {
		sendto(
			sock.as_raw_fd(),
			&msg as *const _ as *const _,
			size_of::<NlUpLinkMsg>(),
			0,
			&addr as *const _ as *const _,
			size_of::<sockaddr_nl>() as _,
		)
	};
	if res < 0 {
		return Err(io::Error::last_os_error().into());
	}
	// Receive response
	let mut msg: MaybeUninit<NlResponse> = MaybeUninit::uninit();
	let res = unsafe {
		recv(
			sock.as_raw_fd(),
			msg.as_mut_ptr() as *mut _,
			size_of::<NlResponse>(),
			0,
		)
	};
	if res < 0 {
		return Err(io::Error::last_os_error().into());
	}
	test_assert!(res as usize >= size_of::<NlResponse>());
	let msg = unsafe { msg.assume_init() };
	test_assert!(msg.hdr.nlmsg_type == NLMSG_ERROR as u16);
	test_assert!(msg.hdr.nlmsg_seq == 1);
	if msg.err.error != 0 {
		return Err(io::Error::from_raw_os_error(-msg.err.error).into());
	}

	// TODO create routes

	Ok(())
}

pub fn ping() -> TestResult {
	exec(Command::new("ping").args(["-c", "3", "github.com"]))
}

pub fn http_call() -> TestResult {
	let client = reqwest::blocking::Client::new();
	let response = client
		.get("http://example.com")
		.timeout(Duration::from_secs(5))
		.send()?;
	test_assert!(response.status().is_success());
	Ok(())
}
