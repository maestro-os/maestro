//! This module handles ACPI's Fixed ACPI Description Table (FADT).

use super::ACPITable;

/// TODO doc
pub struct GenericAddr {
	/// TODO doc
	addr_space: u8,
	/// TODO doc
	bit_width: u8,
	/// TODO doc
	bit_offset: u8,
	/// TODO doc
	access_size: u8,
	/// TODO doc
	address: u8,
}

/// The Fixed ACPI Description Table.
#[repr(C)]
pub struct Fadt {
	/// The signature of the structure.
	signature: [u8; 4],
	/// The length of the structure.
	length: u32,
	/// The revision number of the structure.
	revision: u8,
	/// The checksum to check against all the structure's bytes.
	checksum: u8,
	/// An OEM-supplied string that identifies the OEM.
	oemid: [u8; 6],
	/// TODO doc
	oem_table_id: [u8; 8],
	/// TODO doc
	oemrevision: u32,
	/// TODO doc
	creator_id: u32,
	/// TODO doc
	creator_revision: u32,

	/// TODO doc
	pub firmware_ctrl: u32,
	/// TODO doc
    pub dsdt: u32,

	/// TODO doc
    pub reserved: u8,

	/// TODO doc
    pub preferred_power_management_profile: u8,
	/// TODO doc
    pub sci_interrupt: u16,
	/// TODO doc
    pub smi_commandport: u32,
	/// TODO doc
    pub acpi_enable: u8,
	/// TODO doc
    pub acpi_disable: u8,
	/// TODO doc
    pub s4bios_req: u8,
	/// TODO doc
    pub pstate_control: u8,
	/// TODO doc
    pub pm1a_event_block: u32,
	/// TODO doc
    pub pm1b_event_block: u32,
	/// TODO doc
    pub pm1a_control_block: u32,
	/// TODO doc
    pub pm1b_control_block: u32,
	/// TODO doc
    pub pm2c_ontrolb_lock: u32,
	/// TODO doc
    pub pm_timer_block: u32,
	/// TODO doc
    pub gpe0_block: u32,
	/// TODO doc
    pub gpe1_block: u32,
	/// TODO doc
    pub pm1_event_length: u8,
	/// TODO doc
    pub pm1_control_length: u8,
	/// TODO doc
    pub pm2_control_length: u8,
	/// TODO doc
    pub pm_timer_length: u8,
	/// TODO doc
    pub gpe0_length: u8,
	/// TODO doc
    pub gpe1_length: u8,
	/// TODO doc
    pub gpe1_base: u8,
	/// TODO doc
    pub cstate_control: u8,
	/// TODO doc
    pub worst_c2_latency: u16,
	/// TODO doc
    pub worst_c3_latency: u16,
	/// TODO doc
    pub flush_size: u16,
	/// TODO doc
    pub flush_stride: u16,
	/// TODO doc
    pub duty_offset: u8,
	/// TODO doc
    pub duty_width: u8,
	/// TODO doc
    pub day_alarm: u8,
	/// TODO doc
    pub month_alarm: u8,
	/// TODO doc
    pub century: u8,

	/// TODO doc
    pub boot_architecture_flags: u16,

	/// TODO doc
    pub reserved2: u8,
	/// TODO doc
    pub flags: u32,

	/// TODO doc
    pub reset_reg: GenericAddr,

	/// TODO doc
    pub reset_value: u8,
	/// TODO doc
    pub reserved3: [u8; 3],

	/// TODO doc
    pub x_firmware_control: u64,
	/// TODO doc
    pub x_dsdt: u64,

	/// TODO doc
    pub x_pm1a_event_block: GenericAddr,
	/// TODO doc
    pub x_pm1b_event_block: GenericAddr,
	/// TODO doc
    pub x_pm1a_control_block: GenericAddr,
	/// TODO doc
    pub x_pm1b_control_block: GenericAddr,
	/// TODO doc
    pub x_pm2_control_block: GenericAddr,
	/// TODO doc
    pub x_pm_timer_block: GenericAddr,
	/// TODO doc
    pub x_gpe0_block: GenericAddr,
	/// TODO doc
    pub x_gpe1_block: GenericAddr,
}

impl ACPITable for Fadt {
	fn get_expected_signature() -> [u8; 4] {
		[b'F', b'A', b'D', b'T']
	}

	fn get_signature(&self) -> &[u8; 4] {
		&self.signature
	}

	fn get_length(&self) -> usize {
		self.length as _
	}
}
