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

//! Non-Volatile Memory Express (NVMe) storage driver
//!
//! [NVMe specification](https://nvmexpress.org/wp-content/uploads/NVM-Express-Base-Specification-Revision-2.3-2025.08.01-Ratified.pdf)

use crate::{
	device::{bar::BAR, manager::PhysicalDevice},
	memory::buddy,
	println,
};
use core::hint;
use utils::{errno, errno::EResult, limits::PAGE_SIZE};

/// Register: Controller capabilities
const REG_CAP: usize = 0x00;
/// Register: Controller Configuration
const REG_CC: usize = 0x14;
/// Register: Controller Status
const REG_CSTS: usize = 0x1c;
/// Register: Admin queue attributes
const REG_AQA: usize = 0x24;
/// Register: Admin submission queue
const REG_ASQ: usize = 0x28;
/// Register: Admin completion queue
const REG_ACQ: usize = 0x30;

/// Flag (CC): Enable
const FLAG_CC_EN: u64 = 0b1;
/// Flag (CSTS): Ready
const FLAG_CSTS_RDY: u64 = 0b1;

#[repr(C)]
struct IoQueue {
	addr: u64,
	size: u64,
}

#[repr(C)]
struct SubmissionQueueEntry {
	/// Command
	command: u32,
	/// Namespace (drive) identifier
	nsid: u32,
	_reserved: [u32; 2],
	/// Metadata address
	metadata_addr: [u32; 2],
	/// Data addresses
	data_addr: [u32; 4],
	/// Command-specific values
	command_specific: [u32; 6],
}

#[repr(C)]
struct CompletionQueueEntry {
	/// Command-specific values
	command_specific: u64,
	_reserved: u64,
	/// Submission queue head address
	submission_queue_head: u16,
	/// Submission queue ID
	submission_queue_id: u16,
	/// Command identifier
	cmd_id: u16,
	/// Status
	status: u16,
}

fn wait_rdy(bar: &BAR, rdy: bool) {
	loop {
		let sts = bar.read::<u64>(REG_CSTS);
		if (sts & FLAG_CSTS_RDY != 0) == rdy {
			break;
		}
		hint::spin_loop();
	}
}

/// A NVMe controller.
pub struct Controller {
	/// Base Address Register
	bar: BAR,
}

impl Controller {
	/// Creates a new instance.
	///
	/// If the device is invalid, the function returns `None`.
	pub fn new(dev: &dyn PhysicalDevice) -> EResult<Self> {
		let bar = dev.get_bars().first().cloned().flatten();
		let Some(bar) = bar else {
			println!("NVMe controller does not have a BAR");
			return Err(errno!(EINVAL));
		};
		// Wait any previous reset to be done
		wait_rdy(&bar, false);
		// Initialize ASQ and ACQ. A SQE (64 bytes) is four times larger than a CQE (16 bytes)
		let asq = buddy::alloc_kernel(2, 0)?;
		let acq = buddy::alloc_kernel(0, 0)?;
		let asq_len = (PAGE_SIZE << 2) / size_of::<SubmissionQueueEntry>();
		let acq_len = PAGE_SIZE / size_of::<CompletionQueueEntry>();
		let aqa = (acq_len << 16) | asq_len;
		bar.write::<u64>(REG_ASQ, asq.as_ptr() as u64);
		bar.write::<u64>(REG_ACQ, acq.as_ptr() as u64);
		bar.write::<u32>(REG_AQA, aqa as u64);
		// TODO check/set controller capabilities
		// Enable controller
		bar.write::<u64>(REG_CC, bar.read::<u64>(REG_CC) | FLAG_CC_EN);
		wait_rdy(&bar, true);
		// TODO identify controller
		// TODO check the controller supports I/O submission/completion queues
		// TODO allocate I/O queues
		Ok(Self {
			bar,
		})
	}

	/// Detect drives.
	pub fn detect(&self) {
		// use the identify command to detect namespaces
		todo!()
	}
}
