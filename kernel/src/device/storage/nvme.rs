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

use crate::{
	device::{bar::BAR, manager::PhysicalDevice},
	println,
};

#[repr(C)]
struct IoQueue {
	addr: u64,
	size: u64,
}

#[repr(C)]
struct SubmissionQueueEntry {
	/// Command
	command: u32,
	/// Namespace identifier
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

/// A NVMe controller.
pub struct Controller {
	/// Base Address Register
	bar: BAR,
}

impl Controller {
	/// Creates a new instance.
	///
	/// If the device is invalid, the function returns `None`.
	pub fn new(dev: &dyn PhysicalDevice) -> Option<Self> {
		let bar = dev.get_bars().first().cloned().flatten();
		if let Some(bar) = bar {
			Some(Self {
				bar,
			})
		} else {
			println!("NVMe controller does not have a BAR");
			None
		}
	}

	/// Detect drives.
	pub fn detect(&self) {
		todo!()
	}
}
