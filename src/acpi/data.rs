//! This module implements a structure which allows to retrieve the ACPI data from physical memory.
//!
//! The issue when retrieving such information is that if the system has too much memory, the ACPI
//! data may be too high in memory to recover directly. The structure implemented in this module
//! uses a temporary virtual memory context to get a copy of the data.

use core::ffi::c_void;
use core::intrinsics::wrapping_add;
use core::mem::size_of;
use core::ptr::copy_nonoverlapping;
use crate::acpi::ACPITable;
use crate::acpi::ACPITableHeader;
use crate::acpi::rsdt::Rsdt;
use crate::errno::Errno;
use crate::memory::malloc;
use crate::memory::vmem;
use crate::memory;
use crate::util;

/// The signature of the RSDP structure.
const RSDP_SIGNATURE: &str = "RSD PTR ";

/// Returns the scan range in which is located the RSDP signature.
#[inline(always)]
fn get_scan_range() -> (*const c_void, *const c_void) {
	let begin = (memory::PROCESS_END as usize + 0xe0000) as *const c_void;
	let end = (memory::PROCESS_END as usize + 0xfffff) as *const c_void;

	(begin, end)
}

/// The Root System Description Pointer (RSDP) is a structure storing a pointer to the other
/// structures used by ACPI.
#[repr(C)]
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
		let mut sum: u8 = 0;

		for i in 0..size_of::<Self>() {
			let byte = unsafe { // Safe since every bytes of `s` are readable.
				*((self as *const Self as *const u8 as usize + i) as *const u8)
			};
			sum = wrapping_add(sum, byte);
		}

		sum == 0
	}
}

/// This structure is the version 2.0 of the RSDP. This structure contains the field from the
/// previous version, plus some extra fields.
#[repr(C)]
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
	let (scan_begin, scan_end) = get_scan_range();
	let mut i = scan_begin;

	while i < scan_end {
		if util::memcmp(i, RSDP_SIGNATURE.as_ptr() as _, RSDP_SIGNATURE.len()) == 0 {
			return Some(&mut *(i as *mut Rsdp));
		}

		i = i.add(16);
	}

	None
}

/// Structure containing a copy of the ACPI data read from memory.
pub struct ACPIData {
	/// The offset of the data in the physical memory.
	off: usize,
	/// The pointer in the physical memory to the RSDT.
	rsdt: *const Rsdt,

	/// The buffer containing the ACPI data.
	data: *const u8,
}

impl ACPIData {
	/// Reads the ACPI data from memory and returns a buffer containing it with its offset in
	/// physical memory.
	/// If no ACPI data is found, the function returns None.
	/// If the data is invalid, the function makes the kernel panic.
	pub fn read() -> Result<Option<Self>, Errno> {
		let rsdp = unsafe {
			find_rsdp()
		};
		if rsdp.is_none() {
			return Ok(None);
		}
		let rsdp = rsdp.unwrap();
		if !rsdp.check() {
			crate::kernel_panic!("Invalid ACPI pointer!");
		}

		// Temporary vmem used to read the data.
		let mut tmp_vmem = vmem::new()?;
		let rsdt_phys_ptr = rsdp.rsdt_address as *const c_void;
		let rsdt_map_begin = util::down_align(rsdt_phys_ptr, memory::PAGE_SIZE);
		// Mapping the RSDT to make it readable
		tmp_vmem.map_range(rsdt_map_begin, memory::PAGE_SIZE as _, 2, 0)?;

		tmp_vmem.bind();
		let (off, ptr) = {
			let rsdt_ptr = (memory::PAGE_SIZE
				+ (rsdt_phys_ptr as usize - rsdt_map_begin as usize)) as *const Rsdt;
			let rsdt = unsafe { // Safe because the pointer has been mapped before
				&*rsdt_ptr
			};

			if !rsdt.header.check() {
				crate::kernel_panic!("Invalid ACPI structure!");
			}

			// The lowest physical pointer in the ACPI data
			let mut lowest = rsdt_phys_ptr;
			// The highest physical pointer in the ACPI data
			let mut highest = unsafe {
				(rsdt_phys_ptr as *const c_void).add(rsdt.header.get_length())
			};

			rsdt.foreach_table(| table_ptr | {
				if (table_ptr as *const c_void) < lowest {
					lowest = table_ptr as *const c_void;
				}

				// Mapping the table to read its length
				let table_map_begin = util::down_align(table_ptr, memory::PAGE_SIZE);
				if tmp_vmem.map_range(table_map_begin as _,
					(memory::PAGE_SIZE * 3) as _, 2, 0).is_err() {
					crate::kernel_panic!("Unexpected error when reading ACPI data");
				}

				let table_offset = table_ptr as usize - table_map_begin as usize;
				let table = unsafe { // Safe because the pointer has been mapped before
					&*(((memory::PAGE_SIZE * 3) + table_offset) as *const ACPITableHeader)
				};
				// The end of the table
				let end = unsafe {
					(table_ptr as *const c_void).add(table.get_length())
				};

				if end > highest {
					highest = end;
				}
			});

			// Mapping the full ACPI data
			let begin = util::down_align(lowest, memory::PAGE_SIZE);
			let end = util::align(highest, memory::PAGE_SIZE);
			let pages = (end as usize - begin as usize) / memory::PAGE_SIZE;
			tmp_vmem.map_range(begin, memory::PAGE_SIZE as _, pages, 0)?;

			let size = pages * memory::PAGE_SIZE;
			let dest = unsafe {
				malloc::alloc(size)? as *mut u8
			};
			let src = memory::PAGE_SIZE as *const u8;
			unsafe {
				copy_nonoverlapping(src, dest, size);
			}

			(begin as usize, dest)
		};
		crate::bind_vmem();

		Ok(Some(Self {
			off,
			rsdt: rsdt_phys_ptr as _,

			data: ptr,
		}))
	}

	/// Returns a reference to the ACPI table with type T.
	pub fn get_table<T: ACPITable>(&self) -> Option<&T> {
		let rsdt_ptr = unsafe {
			self.data.add(self.rsdt as usize - self.off) as *const Rsdt
		};
		let rsdt = unsafe { // Safe because the pointer has been mapped before
			&*rsdt_ptr
		};

		let entries_len = rsdt.header.get_length() as usize - size_of::<Rsdt>();
		let entries_count = entries_len / size_of::<u32>();
		let entries_ptr = (rsdt_ptr as usize + size_of::<Rsdt>()) as *const u32;

		for i in 0..entries_count {
			let header_ptr = unsafe {
				(self.data.add(*entries_ptr.add(i) as usize - self.off) as usize)
					as *const ACPITableHeader
			};
			let header = unsafe {
				&*header_ptr
			};

			if *header.get_signature() == T::get_expected_signature() {
				let table_ptr = header_ptr as *const T;
				let table = unsafe {
					&*table_ptr
				};

				return Some(table);
			}
		}

		None
	}
}

impl Drop for ACPIData {
	fn drop(&mut self) {
		unsafe { // Safe because the pointer is valid
			malloc::free(self.data as _);
		}
	}
}
