//! The DSDT (Differentiated System Description Table) provides informations about supported power
//! events.
//!
//! This table contains AML code which has to be parsed and executed to retrieve the required
//! informations.

use super::ACPITable;
use super::ACPITableHeader;
use core::mem::size_of;
use core::slice;

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

		unsafe { slice::from_raw_parts(&self.definition_block[0], code_len) }
	}
}

impl ACPITable for Dsdt {
	fn get_expected_signature() -> &'static [u8; 4] {
		&[b'D', b'S', b'D', b'T']
	}
}
