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

//! Non-Volatile Memory Express (NVMe) storage driver
//!
//! [NVMe specification](https://nvmexpress.org/wp-content/uploads/NVM-Express-Base-Specification-Revision-2.3-2025.08.01-Ratified.pdf)

use crate::{
	arch::core_id,
	device::{
		BlkDev, BlockDeviceOps, DeviceID, bar::Bar, bus::pci::PciDev, manager::PhysicalDevice,
		register_blk,
	},
	int,
	int::CallbackHook,
	memory::{
		VirtAddr, buddy,
		buddy::{FrameOrder, ZONE_KERNEL},
		cache::{FrameOwner, RcFrame},
	},
	println, process,
	process::{Process, State, scheduler::schedule},
	sync::{mutex::Mutex, semaphore::Semaphore},
};
use core::{
	any::Any, array, fmt, fmt::Formatter, hint, hint::unlikely, mem::MaybeUninit, num::NonZeroU64,
	ptr::NonNull,
};
use utils::{
	boxed::Box, collections::path::PathBuf, errno, errno::EResult, limits::PAGE_SIZE, math,
	ptr::arc::Arc,
};

/// Register: Controller capabilities
const REG_CAP: usize = 0x00;
/// Register: Version
const REG_VS: usize = 0x08;
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
/// Flag (CSTS): Controller Fatal Status
const FLAG_CSTS_CFS: u32 = 0b10;

/// Command opcode: Identify
const CMD_IDENTIFY: u32 = 0x6;

/// Controller or Namespace Structure: Namespace
const CNS_NAMESPACE: u32 = 0;
/// Controller or Namespace Structure: Controller
const CNS_CONTROLLER: u32 = 1;
/// Controller or Namespace Structure: Active Namespace ID List
const CNS_NAMESPACE_LIST: u32 = 2;

const ASQ_LEN: usize = (PAGE_SIZE << 2) / size_of::<SubmissionQueueEntry>();
const ACQ_LEN: usize = PAGE_SIZE / size_of::<CompletionQueueEntry>();

/// Interrupt vector
const INT: u32 = 0x22;

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

#[repr(C)]
struct LbaFormat {
	/// Metadata Size
	ms: u16,
	/// LBA Data Size
	lbads: u8,
	/// Relative Performance
	rp: u8,
}

#[repr(C, align(4096))]
struct IdentifyNamespace {
	/// Namespace Size
	nsze: u64,
	/// Namespace Capacity
	ncap: u64,
	/// Namespace Utilization
	nuse: u64,
	/// Namespace Features
	nsfeat: u8,
	/// Number of LBA Formats
	nlbaf: u8,
	/// Formatted LBA Size
	flbas: u8,
	/// Metadata Capabilities
	mc: u8,
	/// End-to-end Data Protection Capabilities
	dpc: u8,
	/// End-to-end Data Protection Type Settings
	dps: u8,
	/// Namespace Multi-path I/O and Namespace Sharing Capabilities
	nmic: u8,
	/// Reservation Capabilities
	rescap: u8,
	/// Format Progress Indicator
	fpi: u8,
	/// Deallocate Logical Block Features
	dlfeat: u8,
	/// Namespace Atomic Write Unit Normal
	nawun: u16,
	/// Namespace Atomic Write Unit Power Fail
	nawupf: u16,
	/// Namespace Atomic Compare & Write Unit
	nacwu: u16,
	/// Namespace Atomic Boundary Size Normal
	nabsn: u16,
	/// Namespace Atomic Boundary Offset
	nabo: u16,
	/// Namespace Atomic Boundary Size Power Fail
	nabspf: u16,
	/// Namespace Optimal I/O Boundary
	noiob: u16,
	/// NVM Capacity
	nvmcap: [u64; 2],
	/// Namespace Preferred Write Granularity
	npwg: u16,
	/// Namespace Preferred Write Alignment
	npwa: u16,
	/// Namespace Preferred Deallocate Granularity
	npdg: u16,
	/// Namespace Preferred Deallocate Alignment
	npda: u16,
	/// Namespace Optimal Write Size
	nows: u16,
	/// Maximum Single Source Range Length
	mssrl: u16,
	/// Maximum Copy Length
	mcl: u32,
	/// Maximum Source Range Count
	msrc: u8,
	/// Key Per I/O Status
	kpios: u8,
	/// Number of Unique Attribute LBA Formats
	nulbaf: u8,
	_reserved0: u8,
	/// Key Per I/O Data Access Alignment and Granularity
	kpiodaag: u32,
	_reserved1: u8,
	/// ANA Group Identifier
	anagrpid: u32,
	_reserved2: [u8; 3],
	/// Namespace Attributes
	nsattr: u8,
	/// NVM Set Identifier
	nvmsetid: u16,
	/// Endurance Group Identifier
	endgid: u16,
	/// Namespace Globally Unique Identifier
	nguid: [u8; 16],
	/// IEEE Extended Unique Identifier
	eui64: u64,

	/// LBA Format Support
	lbaf: [LbaFormat; 64],

	/// Vendor Specific
	vs: [u8; 3712],
}

/// Controller identification response
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

fn wait_rdy(bar: &Bar, r: bool) {
	loop {
		let sts: u32 = unsafe { bar.read(REG_CSTS) };
		if (sts & FLAG_CSTS_RDY != 0) == r {
			break;
		}
		if unlikely(sts & FLAG_CSTS_CFS != 0) {
			break;
		}
		hint::spin_loop();
	}
}

struct NamespaceOps {
	ctrlr: Arc<ControllerInner>,

	blk_size: NonZeroU64,
	blk_count: u64,
}

impl BlockDeviceOps for NamespaceOps {
	fn block_size(&self) -> NonZeroU64 {
		self.blk_size
	}

	fn blocks_count(&self) -> u64 {
		self.blk_count
	}

	fn read_frame(&self, off: u64, order: FrameOrder, owner: FrameOwner) -> EResult<RcFrame> {
		let frame = RcFrame::new(order, ZONE_KERNEL, owner, off)?;
		todo!()
	}

	fn write_pages(&self, off: u64, buf: &[u8]) -> EResult<()> {
		todo!()
	}
}

impl fmt::Debug for NamespaceOps {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_struct("NamespaceOps")
			.field("blk_size", &self.blk_size)
			.field("blk_count", &self.blk_count)
			.finish()
	}
}

struct QueuePairInner {
	/// Submission queues tail
	sq_tail: u32,
	/// Completion queues head
	cq_head: u32,

	completion_phase: bool,
	/// Associated process for each submission entry
	waiting_processes: [Option<Arc<Process>>; ASQ_LEN],
}

impl Default for QueuePairInner {
	fn default() -> Self {
		Self {
			sq_tail: 0,
			cq_head: 0,

			completion_phase: true,
			waiting_processes: array::from_fn::<_, ASQ_LEN, _>(|_| None),
		}
	}
}

struct QueuePair {
	id: usize,

	/// Submission queue
	sq: NonNull<SubmissionQueueEntry>,
	/// Completion queue
	cq: NonNull<CompletionQueueEntry>,

	/// Limits concurrent users of the queues pair
	sem: Semaphore<false>,
	inner: Mutex<QueuePairInner, false>,
}

struct ControllerInner {
	/// Base Address Register
	bar: Bar,
	/// Doorbell Stride
	dstrd: usize,
	admin_qp: QueuePair,
}

impl ControllerInner {
	/// Submits a command, returning when completed
	fn submit_cmd_sync(&self, qp: &QueuePair, mut cmd: SubmissionQueueEntry) {
		// Wait for space in the submission queue
		let _permit = qp.sem.acquire();
		let mut qp_inner = qp.inner.lock();
		let sq_tail = qp_inner.sq_tail;
		// Add command identifier
		cmd.cdw0 = (cmd.cdw0 & !0xffff0000) | ((sq_tail & 0xffff) << 16);
		// Insert in submission queue
		unsafe {
			qp.sq.add(sq_tail as usize).write_volatile(cmd);
		}
		qp_inner.waiting_processes[sq_tail as usize] = Some(Process::current());
		// Update queue tail
		qp_inner.sq_tail = (sq_tail + 1) % (ASQ_LEN as u32);
		unsafe {
			self.bar.write::<u32>(
				queue_doorbell_off(qp.id, false, self.dstrd),
				qp_inner.sq_tail,
			);
		}
		// Wait for completion
		process::set_state(State::Sleeping);
		schedule();
	}
}

/// A NVMe controller.
pub struct Controller {
	inner: Arc<ControllerInner>,
	int_handle: CallbackHook,
}

impl Controller {
	/// Creates a new instance.
	///
	/// If the device is invalid, the function returns `None`.
	pub fn new(dev: &dyn PhysicalDevice) -> EResult<Self> {
		// A NVMe can only be connected to a PCI bus
		let dev: &PciDev = (dev as &dyn Any).downcast_ref().unwrap();
		let bar = dev.get_bars().first().cloned().flatten();
		let Some(bar) = bar else {
			println!("nvme: BAR not found");
			return Err(errno!(EINVAL));
		};
		// Get version
		let version: u32 = unsafe { bar.read(REG_VS) };
		let major = (version >> 16) & 0xffff;
		let minor = (version >> 8) & 0xff;
		let patch = version & 0xff;
		println!("nvme: controller version {major}.{minor}.{patch}");
		// Enable interrupts, bus mastering and memory access
		dev.write_status_command((dev.read_status_command() & !(1 << 10)) | 0o110);
		// Check page size
		let cap: u64 = unsafe { bar.read(REG_CAP) };
		let min_page_size = math::pow2(((cap as usize >> 48) & 0xf) + 12);
		let max_page_size = math::pow2(((cap as usize >> 52) & 0xf) + 12);
		if unlikely(!(min_page_size..=max_page_size).contains(&PAGE_SIZE)) {
			println!(
				"nvme: unsupported page size (min: {min_page_size} max: {max_page_size}, page size: {PAGE_SIZE})"
			);
			return Err(errno!(EINVAL));
		}
		// Setup MSI
		let msi_x = dev.enable_msi_x();
		if let Some(msi_x) = msi_x {
			msi_x
				.set(0, core_id() as _, true, false, INT)
				.inspect_err(|_| println!("nvme: failed to initialize MSI-x"))?;
			println!("nvme: using MSI-X");
		} else {
			println!("nvme: no MSI-X, driver does not support MSI");
			return Err(errno!(EINVAL));
		}
		// Disable controller
		unsafe {
			bar.write(REG_CC, bar.read::<u32>(REG_CC) & !FLAG_CC_EN);
		}
		wait_rdy(&bar, false);
		// Set capabilities
		unsafe {
			let cc = bar.read::<u32>(REG_CC);
			bar.write(REG_CC, (cc & !0x3ff0) | ((PAGE_SIZE.ilog2() - 12) << 7));
		}
		// Initialize ASQ and ACQ. A SQE (64 bytes) is four times larger than a CQE (16 bytes)
		let asq = buddy::alloc_kernel(2, 0)?;
		let acq = buddy::alloc_kernel(0, 0)?;
		// Zero-initialize queues
		unsafe {
			NonNull::slice_from_raw_parts(asq, PAGE_SIZE << 2)
				.as_mut()
				.fill(0);
			NonNull::slice_from_raw_parts(acq, PAGE_SIZE)
				.as_mut()
				.fill(0);
		}
		let aqa = (ACQ_LEN << 16) | ASQ_LEN;
		unsafe {
			bar.write(
				REG_ASQ,
				VirtAddr::from(asq).kernel_to_physical().unwrap().0 as u64,
			);
			bar.write(
				REG_ACQ,
				VirtAddr::from(acq).kernel_to_physical().unwrap().0 as u64,
			);
			bar.write(REG_AQA, aqa as u32);
			// Enable controller
			bar.write(REG_CC, bar.read::<u32>(REG_CC) | FLAG_CC_EN);
		}
		wait_rdy(&bar, true);
		// Check for fatal error
		let status = unsafe { bar.read::<u32>(REG_CSTS) };
		if unlikely(status & FLAG_CSTS_CFS != 0) {
			println!("nvme: fatal error during initialization");
			unsafe {
				buddy::free_kernel(asq.as_ptr(), 2);
				buddy::free_kernel(acq.as_ptr(), 0);
			}
			return Err(errno!(EINVAL));
		}
		// Setup interrupt handler
		let inner = Arc::new(ControllerInner {
			bar,
			dstrd: ((cap >> 32) & 0xf) as usize,
			admin_qp: QueuePair {
				id: 0,

				sq: asq.cast(),
				cq: acq.cast(),

				sem: Semaphore::new(ASQ_LEN),
				inner: Default::default(),
			},
		})?;
		let inner_ = inner.clone();
		let int_handle = int::register_callback(INT, move |_int, _, _, _| {
			let inner = inner_.clone();
			// TODO use the interrupt ID to determine the queue
			let qp = &inner.admin_qp;
			let mut qp_inner = qp.inner.lock();
			let mut any = false;
			loop {
				let cqe = unsafe { qp.cq.add(qp_inner.cq_head as usize).read_volatile() };
				// Check phase bit
				if (cqe.status & 1 != 0) != qp_inner.completion_phase {
					break;
				}
				// Wake up process
				let proc = qp_inner.waiting_processes[cqe.cid as usize].take();
				if let Some(proc) = proc {
					Process::wake_from(&proc, State::Sleeping as _);
				}
				qp_inner.cq_head = (qp_inner.cq_head + 1) % (ACQ_LEN as u32);
				if qp_inner.cq_head == 0 {
					qp_inner.completion_phase = !qp_inner.completion_phase;
				}
				any = true;
			}
			if any {
				unsafe {
					inner.bar.write::<u32>(
						queue_doorbell_off(qp.id, true, inner.dstrd),
						qp_inner.cq_head,
					);
				}
			}
		})?
		.unwrap();
		let controller = Self {
			inner,
			int_handle,
		};
		// Identify controller
		let mut data: MaybeUninit<IdentifyController> = MaybeUninit::uninit();
		let dptr = VirtAddr::from(&mut data).kernel_to_physical().unwrap();
		controller.inner.submit_cmd_sync(
			&controller.inner.admin_qp,
			SubmissionQueueEntry {
				cdw0: CMD_IDENTIFY,
				nsid: 0,
				cdw12: [0; 2],
				mptr: [0; 2],
				dptr: [dptr.0 as _, 0],
				cdw: [CNS_CONTROLLER, 0, 0, 0, 0, 0],
			},
		);
		// List namespaces
		let mut ns_ids: MaybeUninit<[u32; 1024]> = MaybeUninit::uninit();
		let dptr = VirtAddr::from(&mut ns_ids).kernel_to_physical().unwrap();
		controller.inner.submit_cmd_sync(
			&controller.inner.admin_qp,
			SubmissionQueueEntry {
				cdw0: CMD_IDENTIFY,
				nsid: 0,
				cdw12: [0; 2],
				mptr: [0; 2],
				dptr: [dptr.0 as _, 0],
				cdw: [CNS_NAMESPACE_LIST, 0, 0, 0, 0, 0],
			},
		);
		// TODO allocate I/O queues
		let ns_ids = unsafe { ns_ids.assume_init() };
		for i in ns_ids {
			if i == 0 {
				break;
			}
			controller.init_ns(i)?;
		}
		Ok(controller)
	}

	fn init_ns(&self, id: u32) -> EResult<()> {
		let mut ns_id: MaybeUninit<IdentifyNamespace> = MaybeUninit::uninit();
		let dptr = VirtAddr::from(&mut ns_id).kernel_to_physical().unwrap();
		self.inner.submit_cmd_sync(
			&self.inner.admin_qp,
			SubmissionQueueEntry {
				cdw0: CMD_IDENTIFY,
				nsid: id,
				cdw12: [0; 2],
				mptr: [0; 2],
				dptr: [dptr.0 as _, 0],
				cdw: [CNS_NAMESPACE, 0, 0, 0, 0, 0],
			},
		);
		let ns_id = unsafe { ns_id.assume_init() };
		let fidxl = (ns_id.flbas & 0xf) as usize;
		let blk_size = math::pow2(ns_id.lbaf[fidxl].lbads as u64);
		let path = PathBuf::try_from(b"/dev/TODO")?; // TODO
		let blkdev = BlkDev::new(
			DeviceID {
				major: 259, // TODO
				minor: 0,   // TODO
			},
			path,
			0o660,
			Box::new(NamespaceOps {
				ctrlr: self.inner.clone(),

				blk_size: NonZeroU64::new(blk_size).unwrap(),
				blk_count: ns_id.ncap,
			})?,
		)?;
		register_blk(blkdev)?;
		Ok(())
	}
}

// TODO implement shutdown
