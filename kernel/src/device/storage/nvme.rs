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
	device::{bar::Bar, manager::PhysicalDevice},
	memory::{VirtAddr, buddy},
	println,
};
use core::{
	hint, mem,
	ptr::NonNull,
	sync::atomic::{AtomicUsize, Ordering::Release},
};
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
const FLAG_CC_EN: u32 = 0b1;
/// Flag (CSTS): Ready
const FLAG_CSTS_RDY: u32 = 0b1;

/// Command opcode: Identify
const CMD_IDENTIFY: u32 = 0x6;

/// Controller or Namespace Structure: Namespace
const CNS_NAMESPACE: u32 = 0;
/// Controller or Namespace Structure: Controller
const CNS_CONTROLLER: u32 = 1;
/// Controller or Namespace Structure: Active Namespace ID List
const CNS_NAMESPACE_LIST: u32 = 1;

const ASQ_LEN: usize = (PAGE_SIZE << 2) / size_of::<SubmissionQueueEntry>();
const ACQ_LEN: usize = PAGE_SIZE / size_of::<CompletionQueueEntry>();

/// Returns the register offset for the doorbell property of the given `queue`.
///
/// Arguments:
/// - `completion`: if `false`, returns the offset for the *completion* queue, else the
///   *submission* queue
/// - `stride`: the stride defined in the controller's capabilities
#[inline]
fn queue_doorbell_off(queue: usize, completion: bool, stride: usize) -> usize {
	0x1000 + (2 * queue + completion as usize) * (4 << stride)
}

#[repr(C)]
struct IoQueue {
	addr: u64,
	size: u64,
}

#[repr(C)]
struct SubmissionQueueEntry {
	/// Command DWORD 0
	cdw0: u32,
	/// Namespace (drive) identifier
	nsid: u32,
	/// Command DWORD 1-2
	cdw12: [u32; 2],
	/// Metadata Pointer
	mptr: [u32; 2],
	/// Data Pointer
	dptr: [u64; 2],
	/// Command DWORD 10-15
	cdw: [u32; 6],
}

#[repr(C)]
struct CompletionQueueEntry {
	/// Command DWORD 0-1
	cdw01: [u32; 2],
	/// SQ Head Pointer
	sqhd: u16,
	/// SQ Identifier
	sqid: u16,
	/// Command Identifier
	cid: u16,
	/// Status
	status: u16,
}

/// Controller identification response
#[derive(Debug)]
#[repr(C, align(4096))]
struct IdentifyController {
	// Controller Capabilities and Features
	/// PCI Vendor ID
	vid: u16,
	/// PCI Subsystem Vendor ID
	ssvid: u16,
	/// Serial Number
	sn: [u8; 20],
	/// Model Number
	mn: [u8; 40],
	/// Firmware Revision
	fr: [u8; 8],
	/// Recommended Arbitration Burst
	rab: u8,
	/// IEEE OUI Identifier
	ieee: [u8; 3],
	/// Controller Multi-Path and Namespace Sharing Capabilities
	cmic: u8,
	/// Maximum Data Transfer Size
	mdts: u8,
	/// Controller ID
	cntlid: u16,
	/// Version
	ver: u32,
	/// RTD3 Resume Latency
	rtd3r: u32,
	/// RTD3 Entry Latency
	rtd3e: u32,
	/// Optional Asynchronous Events Supported
	oaes: u32,
	/// Controller Attributes
	ctratt: u32,
	/// Read Recover Levels Supported
	rrls: u16,
	/// Boot Partition Capabilities
	bpcap: u8,
	_reserved0: u8,
	/// NVM Subsystem Shutdown Latency
	nssl: u32,
	_reserved1: u16,
	/// Power Loss Signaling Information
	plsi: u8,
	/// Controller Type
	cntrltype: u8,
	/// FRU Globally Unique Identifier
	fguid: [u8; 16],
	/// Command Retry Delay Time 1
	crdt1: u16,
	/// Command Retry Delay Time 1
	crdt2: u16,
	/// Command Retry Delay Time 1
	crdt3: u16,
	/// Controller Reachability Capabilities
	crcap: u8,
	/// Controller Instance Uniquifier
	ciu: u8,
	/// Controller Instance Random Number
	cirn: [u8; 8],
	_reserved2: [u8; 96],
	_reserved3: [u8; 13],
	/// NVM Subsystem Report
	nvmsr: u8,
	/// VPD Write Cycle Information
	vwci: u8,
	/// Management Endpoint Capabilities
	mec: u8,

	// Admin Command Set Attributes & Optional Controller Capabilities
	/// Optional Admin Command Support
	oacs: u16,
	/// Abort Command Limit
	acl: u8,
	/// Asynchronous Event Request Limit
	aerl: u8,
	/// Firmware Updates
	frmw: u8,
	/// Log Page Attributes
	lpa: u8,
	/// Error Log Page Entries
	elpe: u8,
	/// Number of Power States Support
	npss: u8,
	/// Admin Vendor Specific Command Configuration
	avscc: u8,
	/// Autonomous Power State Transition Attributes
	apsta: u8,
	/// Warning Composite Temperature Threshold
	wctemp: u16,
	/// Critical Composite Temperature Threshold
	cctemp: u16,
	/// Maximum Time for Firmware Activation
	mtfa: u16,
	/// Host Memory Buffer Preferred Size
	hmpre: u32,
	/// Host Memory Buffer Minimum Size
	hmmin: u32,
	/// Total NVM Capacity
	tnvmcap: [u64; 2],
	/// Unallocated NVM Capacity
	unvmcap: [u64; 2],
	/// Replay Protected Memory Block Support
	rpmbs: u32,
	/// Extended Device Self-test Time
	edstt: u16,
	/// Device Self-test Options
	dsto: u8,
	/// Firmware Update Granularity
	fwug: u8,
	/// Keep Alive Support
	kas: u16,
	/// Host Controlled Thermal Management Attributes
	hctma: u16,
	/// Minimum Thermal Management Temperature
	mntmt: u16,
	/// Maximum Thermal Management Temperature
	mxtmt: u16,
	/// Sanitize Capabilities
	sanicap: u32,
	/// Host Memory Buffer Minimum Descriptor Entry Size
	hmminds: u32,
	/// Host Memory Maximum Descriptors Entries
	hmmaxd: u16,
	/// NVM Set Identifier Maximum
	nsetidmax: u16,
	/// Endurance Group Identifier Maximum
	endgidmax: u16,
	/// ANA Transition Time
	anatt: u8,
	/// Asymmetric Namespace Access Capabilities
	anacap: u8,
	/// ANA Group Identifier Maximum
	anagrpmax: u32,
	/// Number of ANA Group Identifiers
	nanagrpid: u32,
	/// Persistent Event Log Size
	pels: u32,
	/// Domain Identifier
	did: u16,
	/// Key Per I/O Capabilities
	kpioc: u8,
	_reserved4: u8,
	/// Maximum Processing Time for Firmware Activation Without Reset
	mptfawr: u16,
	_reserved5: u16,
	/// Max Endurance Group Capacity
	megcap: [u64; 2],
	/// Temperature Threshold Hysteresis Attributes
	tmpthha: u8,
	/// Maximum Unlimited Power Attributes
	mupa: u8,
	/// Command Quiesce Time
	cqt: u16,
	/// Configurable Device Personality Attributes
	cdpa: u16,
	/// Maximum Unlimited Power
	mup: u16,
	/// Interval Power Measurement Sample Rate
	ipmsr: u16,
	/// Maximum Stop Measurement Time
	msmt: u16,
	_reserved6: [u8; 116],

	// NVM Command Set Attributes
	/// Submission Queue Entry Size
	sqes: u8,
	/// Completion Queue Entry Size
	cqes: u8,
	/// Maximum Outstanding Commands
	maxcmd: u16,
	/// Number of Namespaces
	nn: u32,
	/// Optional NVM Command Support
	oncs: u16,
	/// Fused Operation Support
	fuses: u16,
	/// Format NVM Attributes
	fna: u8,
	/// Volatile Write Cache
	vwc: u8,
	/// Atomic Write Unit Normal
	awun: u16,
	/// Atomic Write Unit Power Fail
	awupf: u16,
	/// I/O Command Set Vendor Specific Command Configuration
	icsvscc: u8,
	/// Namespace Write Protection Capabilities
	nwpc: u8,
	/// Atomic Compare & Write Unit
	acwu: u16,
	/// Copy Descriptor Formats Supported
	cdfs: u16,
	/// SGL Support
	sgls: u32,
	/// Maximum Number of Allowed Namespaces
	mnan: u32,
	/// Maximum Domain Namespace Attachments
	maxdna: [u64; 2],
	/// Maximum I/O Controller Namespace Attachments
	maxcna: u32,
	/// Optimal Aggregated Queue Depth
	oaqd: u32,
	/// Recommended Host-Initiated Refresh Interval
	rhiri: u8,
	/// Host-Initiated Refresh Time
	hirt: u8,
	/// Controller Maximum Memory Range Tracking Descriptors
	cmmrtd: u16,
	/// NVM Subsystem Maximum Memory Range Tracking Descriptors
	nmmrtd: u16,
	/// Minimum Memory Range Tracking Granularity
	minmrtg: u8,
	/// Maximum Memory Range Tracking Granularity
	maxmrtg: u8,
	/// Tracking Attributes
	trattr: u8,
	_reserved7: u8,
	/// Maximum Controller User Data Migration Queues
	mcudmq: u16,
	/// Maximum NVM Subsystem User Data Migration Queues
	mnsudmq: u16,
	/// Maximum CDQ Memory Ranges
	mcmr: u16,
	/// NVM Subsystem Maximum CDQ Memory Ranges
	nmcmr: u16,
	/// Maximum Controller Data Queue PRP Count
	mcdqpc: u16,
	_reserved8: [u8; 180],
	/// NVM Subsystem NVMe Qualified Name
	subnqn: [u8; 256],
	_reserved9: [u8; 768],

	// Fabric Specific
	/// I/O Queue Command Capsule Supported Size
	ioccsz: u32,
	/// I/O Queue Response Capsule Support Size
	iorcsz: u32,
	/// In Capsule Data Offset
	icdoff: u16,
	/// Fabrics Controller Attributes
	fcatt: u8,
	/// Maximum SGL Data Block Descriptors
	msdbd: u8,
	/// Optional Fabrics Commands Support
	ofcs: u16,
	/// Discovery Controller Type
	dctype: u8,
	/// Cross-Controller Reset Limit
	ccrl: u8,
	_reserved10: [u8; 240],

	// Power State Descriptors
	/// Power State Descriptors
	psd: [[u8; 32]; 32],

	/// Vendor Specific
	vs: [u8; 1024],
}

fn wait_rdy(bar: &Bar) {
	loop {
		let sts = unsafe { bar.read::<u32>(REG_CSTS) };
		if sts & FLAG_CSTS_RDY != 0 {
			break;
		}
		hint::spin_loop();
	}
}

struct QueuePair {
	id: usize,

	/// Submission queue
	sq: NonNull<SubmissionQueueEntry>,
	/// Completion queue
	cq: NonNull<CompletionQueueEntry>,

	/// Submission queues tail
	sq_tail: AtomicUsize,
	/// Completion queues head
	cq_head: AtomicUsize,
}

/// A NVMe controller.
pub struct Controller {
	/// Base Address Register
	bar: Bar,

	/// Doorbell Stride
	dstrd: usize,

	admin_queues: QueuePair,
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
		// Initialize ASQ and ACQ. A SQE (64 bytes) is four times larger than a CQE (16 bytes)
		let asq = buddy::alloc_kernel(2, 0)?;
		let acq = buddy::alloc_kernel(0, 0)?;
		let aqa = (ACQ_LEN << 16) | ASQ_LEN;
		let cap = unsafe {
			bar.write(REG_ASQ, asq.as_ptr() as u64);
			bar.write(REG_ACQ, acq.as_ptr() as u64);
			bar.write(REG_AQA, aqa as u32);
			// Read controller capabilities
			let cap: u64 = bar.read(REG_CAP);
			// Enable controller
			bar.write(REG_CC, bar.read::<u32>(REG_CC) | FLAG_CC_EN);
			cap
		};
		wait_rdy(&bar);
		let controller = Self {
			bar,

			dstrd: ((cap >> 32) & 0xf) as usize,

			admin_queues: QueuePair {
				id: 0,

				sq: asq.cast(),
				cq: acq.cast(),

				sq_tail: AtomicUsize::new(0),
				cq_head: AtomicUsize::new(0),
			},
		};
		// Identify controller
		let mut data: IdentifyController = unsafe { mem::zeroed() };
		let dptr = VirtAddr::from(&mut data).kernel_to_physical().unwrap();
		controller.submit_cmd_sync(
			&controller.admin_queues,
			SubmissionQueueEntry {
				cdw0: CMD_IDENTIFY,
				nsid: 0,
				cdw12: [0; 2],
				mptr: [0; 2],
				dptr: [dptr.0 as _, 0],
				cdw: [CNS_CONTROLLER, 0, 0, 0, 0, 0],
			},
		);
		println!("-> {:?}", data);
		// TODO list namespaces and identify them
		// TODO allocate I/O queues
		Ok(controller)
	}

	/// Submits a command, returning when completed
	fn submit_cmd_sync(&self, queue: &QueuePair, cmd: SubmissionQueueEntry) {
		// Overflow is fine because ASQ_LEN is a power of 2
		let asq_tail = queue.sq_tail.fetch_add(1, Release) % ASQ_LEN;
		unsafe {
			queue.sq.add(asq_tail).write_volatile(cmd);
			self.bar.write::<u32>(
				queue_doorbell_off(queue.id, false, self.dstrd),
				asq_tail as _,
			);
		}
		// TODO wait for completion (sleep)
	}

	/// Detect drives.
	pub fn detect(&self) {
		// use the identify command to detect namespaces
		todo!()
	}
}
