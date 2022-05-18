//! TODO doc

use core::mem::size_of;
use core::slice;
use super::ACPITable;
use super::ACPITableHeader;

/// The Differentiated System Description Table.
#[repr(C)]
#[derive(Debug)]
pub struct Dsdt {
	/// The table's header.
	pub header: ACPITableHeader,

	/// The definition of the AML code.
	definition_block: [u8],
}

impl Dsdt {
	/// Returns a slice to the AML code.
	pub fn get_aml(&self) -> &[u8] {
		let code_len = self.header.length as usize - size_of::<ACPITableHeader>();

		unsafe {
			slice::from_raw_parts(&self.definition_block[0], code_len)
		}
	}
}

impl ACPITable for Dsdt {
	fn get_expected_signature() -> [u8; 4] {
		[b'D', b'S', b'D', b'T']
	}
}
