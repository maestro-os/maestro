//! This module implements a structure which allows to retrieve the ACPI data
//! from physical memory.
//!
//! The issue when retrieving such information is that if the system has too
//! much memory, the ACPI data may be too high in memory to recover directly.
//!
//! The structure implemented in this module uses a temporary virtual memory
//! context to get a copy of the data.

use crate::acpi::rsdt::Rsdt;
use crate::acpi::ACPITable;
use crate::acpi::ACPITableHeader;
use crate::errno::Errno;
use crate::memory;
use crate::memory::malloc;
use crate::memory::vmem;
use crate::util;
use crate::util::boxed::Box;
use crate::util::container::hashmap::HashMap;
use core::ffi::c_void;
use core::mem::size_of;
use core::ptr;
use core::ptr::copy_nonoverlapping;
use core::ptr::Pointee;
use core::slice;

/// The signature of the RSDP structure.
const RSDP_SIGNATURE: &[u8] = b"RSD PTR ";

/// Returns the scan range in which is located the RSDP signature.
#[inline(always)]
fn get_scan_range() -> (*const c_void, *const c_void) {
	let begin = (memory::PROCESS_END as usize + 0xe0000) as *const c_void;
	let end = (memory::PROCESS_END as usize + 0xfffff) as *const c_void;

	(begin, end)
}

/// The Root System Description Pointer (RSDP) is a structure storing a pointer
/// to the other structures used by ACPI.
#[repr(C)]
#[derive(Debug)]
struct Rsdp {
	/// The signature of the structure.
	signature: [u8; 8],
	/// The checksum to check against all the structure's bytes.
	checksum: u8,
	/// An OEM-supplied string that identifies the OEM.
	oemid: [u8; 6],
	/// The revision number of the structure.
	revision: u8,
	/// The address to the RSDT.
	rsdt_address: u32,
}

impl Rsdp {
	/// Checks that the table is valid.
	pub fn check(&self) -> bool {
		if self.signature != RSDP_SIGNATURE {
			return false;
		}

		let mut sum: u8 = 0;

		for i in 0..size_of::<Self>() {
			let byte = unsafe {
				// Safe since every bytes of `self` are readable.
				*(self as *const Self as *const u8).add(i)
			};
			sum = sum.wrapping_add(byte);
		}

		sum == 0
	}
}

/// This structure is the version 2.0 of the RSDP.
///
/// This structure contains the field from the previous version, plus some extra fields.
#[repr(C)]
#[derive(Debug)]
struct Rsdp2 {
	/// The version 1.0 on structure.
	rsdp: Rsdp,

	/// The length of the structure.
	length: u32,
	/// The address to the XSDT.
	xsdt_address: u64,
	/// The checksum to check against all the structure's bytes.
	extended_checksum: u8,
	/// Reserved bytes that must not be written.
	reserved: [u8; 3],
}

/// Finds the RSDP and returns a reference to it.
unsafe fn find_rsdp() -> Option<&'static mut Rsdp> {
	let (begin, end) = get_scan_range();
	let mut ptr = begin;

	while ptr < end {
		let signature_slice = slice::from_raw_parts(ptr as *const u8, RSDP_SIGNATURE.len());

		if signature_slice == RSDP_SIGNATURE {
			return Some(&mut *(ptr as *mut Rsdp));
		}

		ptr = ptr.add(16);
	}

	None
}

/// Structure containing a copy of the ACPI data read from memory.
#[derive(Debug)]
pub struct ACPIData {
	/// The offset of the data in the physical memory.
	off: usize,
	/// The pointer in the physical memory to the RSDT.
	rsdt: *const Rsdt,

	/// The buffer containing the ACPI data.
	data: malloc::Alloc<u8>,
}

impl ACPIData {
	// TODO Clean
	/// Reads the ACPI data from memory and returns a buffer containing it with its offset in
	/// physical memory.
	///
	/// If no ACPI data is found, the function returns `None`.
	///
	/// If the data is invalid, the function makes the kernel panic.
	pub fn read() -> Result<Option<Self>, Errno> {
		let rsdp = unsafe { find_rsdp() };
		if rsdp.is_none() {
			return Ok(None);
		}
		let rsdp = rsdp.unwrap();
		if !rsdp.check() {
			crate::kernel_panic!("Invalid ACPI pointer!");
		}

		// Temporary vmem used to read the data, since it cannot be located anywhere on the
		// physical memory.
		let mut tmp_vmem = vmem::new()?;
		let rsdt_phys_ptr = rsdp.rsdt_address as *const c_void;
		let rsdt_map_begin = util::down_align(rsdt_phys_ptr, memory::PAGE_SIZE);
		crate::println!("being: {:p}", rsdt_map_begin); // TODO rm
												// Mapping the RSDT to make it readable
		tmp_vmem.map_range(rsdt_map_begin, memory::PAGE_SIZE as _, 2, 0)?;

		tmp_vmem.bind();
		let (off, data) = {
			let rsdt_ptr = (memory::PAGE_SIZE + (rsdt_phys_ptr as usize - rsdt_map_begin as usize))
				as *const Rsdt;
			let rsdt = unsafe {
				// Safe because the pointer has been mapped before
				&*rsdt_ptr
			};

			if !rsdt.header.check::<Rsdt>() {
				crate::kernel_panic!("Invalid ACPI structure!");
			}

			// Getting every ACPI tables
			let mut tables = HashMap::new();
			rsdt.foreach_table(|table_ptr| {
				// Mapping the table to read its length
				let table_map_begin = util::down_align(table_ptr, memory::PAGE_SIZE);
				if tmp_vmem
					.map_range(table_map_begin as _, (memory::PAGE_SIZE * 3) as _, 2, 0)
					.is_err()
				{
					crate::kernel_panic!("Unexpected error when reading ACPI data");
				}

				let table_offset = table_ptr as usize - table_map_begin as usize;
				let table = unsafe {
					// Safe because the pointer has been mapped before
					&*(((memory::PAGE_SIZE * 3) + table_offset) as *const ACPITableHeader)
				};

				let b = unsafe {
					let ptr = malloc::alloc(table.get_length()).unwrap();
					ptr::copy_nonoverlapping(
						table as *const _ as *const _,
						ptr,
						table.get_length(),
					);

					Box::from_raw(ptr as *mut ())
				};
				tables.insert(table.get_signature(), b).unwrap();
			});

			// FIXME: This is garbage due to a merge in order to make things compile.
			// This whole function needs a rewrite
			let lowest = core::ptr::null::<c_void>();
			let highest = core::ptr::null::<c_void>();

			// Mapping the full ACPI data
			let begin = util::down_align(lowest, memory::PAGE_SIZE);
			let end = util::align(highest, memory::PAGE_SIZE);
			let pages = (end as usize - begin as usize) / memory::PAGE_SIZE;
			tmp_vmem.map_range(begin, memory::PAGE_SIZE as _, pages, 0)?;

			let size = pages * memory::PAGE_SIZE;
			let mut dest = malloc::Alloc::<u8>::new_default(size)?;
			let src = memory::PAGE_SIZE as *const u8;
			unsafe {
				copy_nonoverlapping(src, dest.as_ptr_mut(), size);
			}

			(begin as usize, dest)
		};
		crate::bind_vmem();

		Ok(Some(Self {
			off,
			rsdt: rsdt_phys_ptr as _,

			data,
		}))
	}

	// TODO Minimize duplicate code between get_table_*

	/// Returns a reference to the ACPI table with type `T`.
	///
	/// If the table doesn't exist, the function returns `None`.
	pub fn get_table_sized<T: ACPITable>(&self) -> Option<&T> {
		let rsdt_ptr =
			unsafe { self.data.as_ptr().add(self.rsdt as usize - self.off) as *const Rsdt };
		let rsdt = unsafe {
			// Safe because the pointer has been mapped before
			&*rsdt_ptr
		};

		let entries_len = rsdt.header.get_length() - size_of::<Rsdt>();
		let entries_count = entries_len / size_of::<u32>();
		let entries_ptr = (rsdt_ptr as usize + size_of::<Rsdt>()) as *const u32;

		for i in 0..entries_count {
			let header_ptr = unsafe {
				(self
					.data
					.as_ptr()
					.add(*entries_ptr.add(i) as usize - self.off) as usize) as *const ACPITableHeader
			};
			let header = unsafe { &*header_ptr };

			if *header.get_signature() == *T::get_expected_signature() {
				let table = unsafe {
					let table_ptr = header_ptr as *const T;
					&*table_ptr
				};
				if !table.get_header().check::<T>() {
					crate::kernel_panic!("Invalid ACPI structure!");
				}

				return Some(table);
			}
		}

		None
	}

	/// Returns a reference to the ACPI table with type `T`.
	///
	/// The table must be `Unsized`.
	///
	/// If the table doesn't exist, the function returns `None`.
	pub fn get_table_unsized<T: ACPITable + ?Sized + Pointee<Metadata = usize>>(
		&self,
	) -> Option<&T> {
		let rsdt_ptr =
			unsafe { self.data.as_ptr().add(self.rsdt as usize - self.off) as *const Rsdt };
		let rsdt = unsafe {
			// Safe because the pointer has been mapped before
			&*rsdt_ptr
		};

		let entries_len = rsdt.header.get_length() - size_of::<Rsdt>();
		let entries_count = entries_len / size_of::<u32>();
		let entries_ptr = (rsdt_ptr as usize + size_of::<Rsdt>()) as *const u32;

		for i in 0..entries_count {
			let header_ptr = unsafe {
				(self
					.data
					.as_ptr()
					.add(*entries_ptr.add(i) as usize - self.off) as usize) as *const ACPITableHeader
			};
			let header = unsafe { &*header_ptr };

			if *header.get_signature() == *T::get_expected_signature() {
				let len = header.get_length();
				let table = unsafe {
					let table_ptr = ptr::from_raw_parts::<T>(header_ptr as *const (), len);
					&*table_ptr
				};

				if !table.get_header().check::<T>() {
					crate::kernel_panic!("Invalid ACPI structure!");
				}

				return Some(table);
			}
		}

		None
	}
}
