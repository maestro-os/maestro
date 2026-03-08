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
	arch::{core_id, x86::idt::disable_int},
	device::{
		BlkDev, BlockDeviceOps, DeviceID,
		bar::Bar,
		bus::pci::PciDev,
		id::{BLOCK_EXTENDED_MAJOR, BLOCK_EXTENDED_MAJOR_HANDLE},
		manager::PhysicalDevice,
		register_blk,
		storage::{STORAGE_MODE, partition::read_partitions},
	},
	int,
	int::CallbackHandle,
	memory::{VirtAddr, buddy, cache::RcPage},
	println, process,
	process::{Process, State, scheduler::schedule},
	sync::{rwlock::RwLock, semaphore::Semaphore, spin::Spin},
};
use core::{
	any::Any,
	array, fmt,
	fmt::Formatter,
	hint,
	hint::unlikely,
	mem,
	mem::MaybeUninit,
	num::NonZeroU64,
	ptr::NonNull,
	sync::atomic::{AtomicU32, Ordering::Relaxed},
};
use utils::{
	boxed::Box,
	collections::{path::PathBuf, vec::Vec},
	errno,
	errno::{AllocResult, EResult},
	format,
	limits::PAGE_SIZE,
	math,
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

/// Admin command opcode: Create I/O Submission Queue
const ADMIN_CMD_CREATE_IO_SQ: u32 = 0x1;
/// Admin command opcode: Create I/O Completion Queue
const ADMIN_CMD_CREATE_IO_CQ: u32 = 0x5;
/// Admin command opcode: Identify
const ADMIN_CMD_IDENTIFY: u32 = 0x6;

/// Command opcode: Write
const CMD_WRITE: u32 = 0x1;
/// Command opcode: Read
const CMD_READ: u32 = 0x2;

/// Controller or Namespace Structure: Namespace
const CNS_NAMESPACE: u32 = 0;
/// Controller or Namespace Structure: Controller
const CNS_CONTROLLER: u32 = 1;
/// Controller or Namespace Structure: Active Namespace ID List
const CNS_NAMESPACE_LIST: u32 = 2;

const SQ_LEN: usize = (PAGE_SIZE << 2) / size_of::<SubmissionQueueEntry>();
const CQ_LEN: usize = PAGE_SIZE / size_of::<CompletionQueueEntry>();

/// Admin queue interrupt vector
const ADMIN_INT: u32 = 0x22;
/// I/O queue interrupt vector
const IO_INT: u32 = 0x23;

/// Returns the register offset for the doorbell property of the given `queue`.
///
/// Arguments:
/// - `completion`: if `false`, returns the offset for the *completion* queue, else the
///   *submission* queue
/// - `stride`: the stride defined in the controller's capabilities
#[inline]
fn queue_doorbell_off(queue: u16, completion: bool, stride: usize) -> usize {
	0x1000 + (2 * queue as usize + completion as usize) * (4 << stride)
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

impl CompletionQueueEntry {
	#[inline]
	fn status(&self) -> u16 {
		self.status >> 1
	}
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
	_reserved6: [u8; 180],

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
	nsid: u32,
}

impl BlockDeviceOps for NamespaceOps {
	fn new_partition(&self, _dev: &BlkDev, id: u32) -> AllocResult<(DeviceID, PathBuf)> {
		let dev_id = DeviceID {
			major: BLOCK_EXTENDED_MAJOR,
			minor: BLOCK_EXTENDED_MAJOR_HANDLE.lock().alloc_minor(None)?,
		};
		let path = PathBuf::new_unchecked(format!(
			"/dev/nvme{ctrlr_id}n{nsid}p{id}",
			ctrlr_id = self.ctrlr.id,
			nsid = self.nsid
		)?);
		Ok((dev_id, path))
	}

	fn drop_partition(&self, dev: &BlkDev) {
		if dev.id.major == BLOCK_EXTENDED_MAJOR {
			BLOCK_EXTENDED_MAJOR_HANDLE.lock().free_minor(dev.id.minor);
		}
	}

	fn read_page(&self, dev: &Arc<BlkDev>, off: u64) -> EResult<RcPage> {
		let blocks = PAGE_SIZE as u64 / dev.blk_size.get();
		let lba = off * blocks;
		// Bound check
		let end_lba = lba.checked_add(blocks).ok_or_else(|| errno!(EOVERFLOW))?;
		if unlikely(end_lba > dev.blk_count) {
			return Err(errno!(EOVERFLOW));
		}
		dev.mapped.get_or_insert_page(off, || {
			let blk = BlkDev::new_page(dev, off)?;
			let qp = &self.ctrlr.queues.read()[0];
			let cqe = self.ctrlr.submit_cmd_sync(
				qp,
				SubmissionQueueEntry {
					cdw0: CMD_READ,
					nsid: self.nsid,
					cdw12: [0, 0],
					mptr: [0, 0],
					dptr: [blk.phys_addr().0 as _, 0],
					cdw: [lba as u32, (lba >> 32) as u32, (blocks - 1) as _, 0, 0, 0],
				},
			);
			if unlikely(cqe.status() != 0) {
				// TODO print log?
				return Err(errno!(EIO));
			}
			Ok(blk)
		})
	}

	fn writeback(&self, dev: &BlkDev, off: u64, blk: &RcPage) -> EResult<()> {
		let blocks = PAGE_SIZE as u64 / dev.blk_size.get();
		let lba = off * blocks;
		// Bound check
		let end_lba = lba.checked_add(blocks).ok_or_else(|| errno!(EOVERFLOW))?;
		if unlikely(end_lba > dev.blk_count) {
			return Err(errno!(EOVERFLOW));
		}
		let qp = &self.ctrlr.queues.read()[0];
		let cqe = self.ctrlr.submit_cmd_sync(
			qp,
			SubmissionQueueEntry {
				cdw0: CMD_WRITE,
				nsid: self.nsid,
				cdw12: [0, 0],
				mptr: [0, 0],
				dptr: [blk.phys_addr().0 as _, 0],
				cdw: [lba as u32, (lba >> 32) as u32, (blocks - 1) as _, 0, 0, 0],
			},
		);
		if unlikely(cqe.status() != 0) {
			// TODO print log?
			return Err(errno!(EIO));
		}
		Ok(())
	}
}

impl fmt::Debug for NamespaceOps {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_struct("NamespaceOps")
			.field("nsid", &self.nsid)
			.finish()
	}
}

enum QueueEntry {
	Empty,
	Submitted(Arc<Process>),
	Completed(CompletionQueueEntry),
}

struct QueuePairInner {
	/// Submission queues tail
	sq_tail: u32,
	/// Completion queues head
	cq_head: u32,

	completion_phase: bool,
	/// Associated data for each submission entry
	entries: [QueueEntry; SQ_LEN],
}

impl Default for QueuePairInner {
	fn default() -> Self {
		Self {
			sq_tail: 0,
			cq_head: 0,

			completion_phase: true,
			entries: array::from_fn::<_, SQ_LEN, _>(|_| QueueEntry::Empty),
		}
	}
}

struct QueuePair {
	id: u16,

	/// Submission queue
	sq: NonNull<SubmissionQueueEntry>,
	/// Completion queue
	cq: NonNull<CompletionQueueEntry>,

	/// Limits concurrent users of the queues pair
	sem: Semaphore<false>,
	inner: Spin<QueuePairInner, false>,
}

impl QueuePair {
	/// Allocates space for a queue pair and returns the associated instance
	pub fn new(id: u16) -> AllocResult<Self> {
		// A SQE (64 bytes) is four times larger than a CQE (16 bytes)
		let sq = buddy::alloc_kernel(2, 0)?;
		let cq = buddy::alloc_kernel(0, 0)?; // TODO on failure, free previous
		// Zero-initialize queues
		unsafe {
			NonNull::slice_from_raw_parts(sq, PAGE_SIZE << 2)
				.as_mut()
				.fill(0);
			NonNull::slice_from_raw_parts(cq, PAGE_SIZE)
				.as_mut()
				.fill(0);
		}
		Ok(Self {
			id,

			sq: sq.cast(),
			cq: cq.cast(),

			sem: Semaphore::new(SQ_LEN),
			inner: Default::default(),
		})
	}
}

impl Drop for QueuePair {
	fn drop(&mut self) {
		unsafe {
			buddy::free_kernel(self.sq.cast().as_ptr(), 2);
			buddy::free_kernel(self.cq.cast().as_ptr(), 0);
		}
	}
}

struct ControllerInner {
	/// Controller device ID
	id: u32,
	/// Base Address Register
	bar: Bar,
	/// Doorbell Stride
	dstrd: usize,

	admin_qp: QueuePair,
	/// I/O queues list
	queues: RwLock<Vec<QueuePair>>,
}

impl ControllerInner {
	fn init_ns(this: &Arc<Self>, nsid: u32) -> EResult<()> {
		// Build device path
		let path =
			PathBuf::new_unchecked(format!("/dev/nvme{ctrlr_id}n{nsid}", ctrlr_id = this.id)?);
		println!("nvme: detected namespace ({path})");
		let mut ns_id: MaybeUninit<IdentifyNamespace> = MaybeUninit::uninit();
		let dptr = VirtAddr::from(&mut ns_id).kernel_to_physical().unwrap();
		let cqe = this.submit_cmd_sync(
			&this.admin_qp,
			SubmissionQueueEntry {
				cdw0: ADMIN_CMD_IDENTIFY,
				nsid,
				cdw12: [0; 2],
				mptr: [0; 2],
				dptr: [dptr.0 as _, 0],
				cdw: [CNS_NAMESPACE, 0, 0, 0, 0, 0],
			},
		);
		let ns_id = unsafe { ns_id.assume_init() };
		if unlikely(cqe.status() != 0) {
			println!("nvme: failed to identify namespace {nsid}");
			return Ok(());
		}
		// Determine block size
		let fidxl = (ns_id.flbas & 0xf) as usize;
		let blk_size = math::pow2(ns_id.lbaf[fidxl].lbads as u64);
		if unlikely(blk_size > PAGE_SIZE as u64) {
			println!("nvme: unsupported block size {blk_size} on namespace {nsid}");
			return Ok(());
		}
		// Allocate minor
		let id = {
			let mut major = BLOCK_EXTENDED_MAJOR_HANDLE.lock();
			let minor = major.alloc_minor(None)?;
			DeviceID {
				major: major.get_major(),
				minor,
			}
		};
		// Register devices
		let blkdev = BlkDev::new(
			id,
			path,
			STORAGE_MODE,
			NonZeroU64::new(blk_size).unwrap(),
			ns_id.ncap,
			Box::new(NamespaceOps {
				ctrlr: this.clone(),
				nsid,
			})?,
		)?;
		register_blk(blkdev.clone())?;
		read_partitions(&blkdev)?;
		Ok(())
	}

	fn init_io_queue(&self, id: u16, int: u16) -> EResult<()> {
		let qp = QueuePair::new(id)?;
		let dptr = VirtAddr::from(qp.cq).kernel_to_physical().unwrap();
		let cqe = self.submit_cmd_sync(
			&self.admin_qp,
			SubmissionQueueEntry {
				cdw0: ADMIN_CMD_CREATE_IO_CQ,
				nsid: 0,
				cdw12: [0; 2],
				mptr: [0; 2],
				dptr: [dptr.0 as _, 0],
				cdw: [
					(id as u32) | ((CQ_LEN - 1) << 16) as u32,
					0b11 | ((int as u32) << 16),
					0,
					0,
					0,
					0,
				],
			},
		);
		if unlikely(cqe.status() != 0) {
			println!(
				"nvme: I/O completion queue {id} creation error ({})",
				cqe.status()
			);
			return Err(errno!(EIO));
		}
		let dptr = VirtAddr::from(qp.sq).kernel_to_physical().unwrap();
		let cqe = self.submit_cmd_sync(
			&self.admin_qp,
			SubmissionQueueEntry {
				cdw0: ADMIN_CMD_CREATE_IO_SQ,
				nsid: 0,
				cdw12: [0; 2],
				mptr: [0; 2],
				dptr: [dptr.0 as _, 0],
				cdw: [
					(id as u32) | ((SQ_LEN - 1) << 16) as u32,
					0b1 | ((id as u32) << 16),
					0,
					0,
					0,
					0,
				],
			},
		);
		if unlikely(cqe.status() != 0) {
			println!(
				"nvme: I/O submission queue {id} creation error ({})",
				cqe.status()
			);
			return Err(errno!(EIO));
		}
		self.queues.write().push(qp)?;
		Ok(())
	}

	/// Submits a command, returning when completed
	#[must_use]
	fn submit_cmd_sync(
		&self,
		qp: &QueuePair,
		mut cmd: SubmissionQueueEntry,
	) -> CompletionQueueEntry {
		// Wait for space in the submission queue
		let _permit = qp.sem.acquire();
		// Disable interrupts to prevent the completion interrupt from being handled before the
		// process is put to sleep
		disable_int(|| {
			let sq_tail;
			{
				let mut qp_inner = qp.inner.lock();
				sq_tail = qp_inner.sq_tail;
				debug_assert!(matches!(
					qp_inner.entries[sq_tail as usize],
					QueueEntry::Empty
				));
				// Add command identifier
				cmd.cdw0 = (cmd.cdw0 & !0xffff0000) | ((sq_tail & 0xffff) << 16);
				// Insert in submission queue
				unsafe {
					qp.sq.add(sq_tail as usize).write_volatile(cmd);
				}
				qp_inner.entries[sq_tail as usize] = QueueEntry::Submitted(Process::current());
				// Update queue tail
				qp_inner.sq_tail = (sq_tail + 1) % (SQ_LEN as u32);
				unsafe {
					self.bar.write::<u32>(
						queue_doorbell_off(qp.id, false, self.dstrd),
						qp_inner.sq_tail,
					);
				}
				// Wait for completion
				process::set_state(State::Sleeping);
			}
			schedule();
			// Retrieve CQE
			let mut qp_inner = qp.inner.lock();
			let ent = mem::replace(&mut qp_inner.entries[sq_tail as usize], QueueEntry::Empty);
			let QueueEntry::Completed(cqe) = ent else {
				panic!();
			};
			cqe
		})
	}
}

fn handle_int(int: u32, inner: &ControllerInner) {
	let queues;
	let qp = match int {
		ADMIN_INT => &inner.admin_qp,
		IO_INT.. => {
			let i = int - IO_INT;
			queues = inner.queues.read();
			let Some(qp) = queues.get(i as usize) else {
				return;
			};
			qp
		}
		_ => return,
	};
	let mut qp_inner = qp.inner.lock();
	let mut any = false;
	loop {
		let cqe = unsafe { qp.cq.add(qp_inner.cq_head as usize).read_volatile() };
		// Check phase bit
		if (cqe.status & 1 != 0) != qp_inner.completion_phase {
			break;
		}
		// Wake up process
		let ent = mem::replace(
			&mut qp_inner.entries[cqe.cid as usize],
			QueueEntry::Completed(cqe),
		);
		let QueueEntry::Submitted(proc) = ent else {
			unreachable!();
		};
		Process::wake_from(&proc, State::Sleeping as _);
		qp_inner.cq_head = (qp_inner.cq_head + 1) % (CQ_LEN as u32);
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
}

/// A NVMe controller.
pub struct Controller {
	inner: Arc<ControllerInner>,

	admin_int: CallbackHandle,
	io_int: CallbackHandle,
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
		let min_page_size = math::pow2(((cap >> 48) & 0xf) + 12) as usize;
		let max_page_size = math::pow2(((cap >> 52) & 0xf) + 12) as usize;
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
				.set(0, core_id() as _, true, false, ADMIN_INT)
				.inspect_err(|_| println!("nvme: failed to initialize MSI-x"))?;
			msi_x
				.set(1, core_id() as _, true, false, IO_INT)
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
		let admin_qp = QueuePair::new(0)?;
		let aqa = (CQ_LEN << 16) | SQ_LEN;
		unsafe {
			bar.write(
				REG_ASQ,
				VirtAddr::from(admin_qp.sq).kernel_to_physical().unwrap().0 as u64,
			);
			bar.write(
				REG_ACQ,
				VirtAddr::from(admin_qp.cq).kernel_to_physical().unwrap().0 as u64,
			);
			bar.write(REG_AQA, aqa as u32);
			// Set capabilities
			let mut cc = bar.read::<u32>(REG_CC);
			// Set submission and completion queues entry sizes now to circumvent a bug on some
			// QEMU versions
			cc |= 6 << 16;
			cc |= 4 << 20;
			// Set page size
			cc = (cc & !0x3ff0) | ((PAGE_SIZE.ilog2() - 12) << 7);
			// Enable controller
			bar.write(REG_CC, cc | FLAG_CC_EN);
		}
		wait_rdy(&bar, true);
		// Check for fatal error
		let status = unsafe { bar.read::<u32>(REG_CSTS) };
		if unlikely(status & FLAG_CSTS_CFS != 0) {
			println!("nvme: fatal error during initialization");
			return Err(errno!(EINVAL));
		}
		// Setup interrupt handler
		static CTRLR_ID: AtomicU32 = AtomicU32::new(0);
		let inner = Arc::new(ControllerInner {
			id: CTRLR_ID.fetch_add(1, Relaxed),
			bar,
			dstrd: ((cap >> 32) & 0xf) as usize,
			admin_qp,
			queues: RwLock::new(Vec::new()),
		})?;
		let inner_ = inner.clone();
		let (admin_int, io_int) = unsafe {
			let admin_int = int::register_callback(ADMIN_INT, move |int, _, _, _| {
				let inner = inner_.clone();
				handle_int(int, &inner);
			})?
			.unwrap();
			let inner_ = inner.clone();
			let io_int = int::register_callback(IO_INT, move |int, _, _, _| {
				let inner = inner_.clone();
				handle_int(int, &inner);
			})?
			.unwrap();
			(admin_int, io_int)
		};
		// Identify controller
		let mut ctrlr_id: MaybeUninit<IdentifyController> = MaybeUninit::uninit();
		let dptr = VirtAddr::from(&mut ctrlr_id).kernel_to_physical().unwrap();
		let cqe = inner.submit_cmd_sync(
			&inner.admin_qp,
			SubmissionQueueEntry {
				cdw0: ADMIN_CMD_IDENTIFY,
				nsid: 0,
				cdw12: [0; 2],
				mptr: [0; 2],
				dptr: [dptr.0 as _, 0],
				cdw: [CNS_CONTROLLER, 0, 0, 0, 0, 0],
			},
		);
		if unlikely(cqe.status() != 0) {
			println!(
				"nvme: controller identification failed (status: {})",
				cqe.status()
			);
			return Err(errno!(EIO));
		}
		let ctrlr_id = unsafe { ctrlr_id.assume_init() };
		// Check entries sizes
		let minsubsize = (ctrlr_id.sqes & 0xf) as u32;
		let maxsubsize = ((ctrlr_id.sqes >> 4) & 0xf) as u32;
		let mincomsize = (ctrlr_id.cqes & 0xf) as u32;
		let maxcomsize = ((ctrlr_id.cqes >> 4) & 0xf) as u32;
		let subsize = size_of::<SubmissionQueueEntry>().ilog2();
		let comsize = size_of::<CompletionQueueEntry>().ilog2();
		if unlikely(
			!(minsubsize..=maxsubsize).contains(&subsize)
				|| !(mincomsize..=maxcomsize).contains(&comsize),
		) {
			println!("nvme: unsupported queue entry size");
			return Err(errno!(EINVAL));
		}
		unsafe {
			inner.bar.write(
				REG_CC,
				inner.bar.read::<u32>(REG_CC) | (subsize << 16) | (comsize << 20),
			);
		}
		// List namespaces
		let mut ns_ids: MaybeUninit<[u32; 1024]> = MaybeUninit::uninit();
		let dptr = VirtAddr::from(&mut ns_ids).kernel_to_physical().unwrap();
		let cqe = inner.submit_cmd_sync(
			&inner.admin_qp,
			SubmissionQueueEntry {
				cdw0: ADMIN_CMD_IDENTIFY,
				nsid: 0,
				cdw12: [0; 2],
				mptr: [0; 2],
				dptr: [dptr.0 as _, 0],
				cdw: [CNS_NAMESPACE_LIST, 0, 0, 0, 0, 0],
			},
		);
		if unlikely(cqe.status() != 0) {
			println!("nvme: namespace listing failed (status: {})", cqe.status());
			return Err(errno!(EIO));
		}
		let dev_path = PathBuf::new_unchecked(format!("/dev/nvme{}", inner.id)?);
		println!("nvme: detected controller ({dev_path})");
		let controller = Self {
			inner,
			admin_int,
			io_int,
		};
		controller.inner.init_io_queue(1, 1)?;
		let ns_ids = unsafe { ns_ids.assume_init() };
		for i in ns_ids {
			if i == 0 {
				break;
			}
			ControllerInner::init_ns(&controller.inner, i)?;
		}
		// TODO create char device file for the controller
		Ok(controller)
	}
}

// TODO implement shutdown (delete device files)
