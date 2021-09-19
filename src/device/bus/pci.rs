//! The PCI (Peripheral Component Interconnect) is a bus which allows to attach hardware devices on
//! the motherboard. There here-module allows to retrieve informations on the devices attached to
//! the computer's PCI.
//!
//! The device ID, vendor ID, class and subclass of a device allows to determine which driver is
//! required for the device.
//!
//! A PCI device can specify one or several BARs (Base Address Registers). They specify the address
//! of the device's registers in memory, allowing communications through DMA (Direct Memory
//! Access).

use core::mem::size_of;
use crate::device::manager::PhysicalDevice;
use crate::io;
use crate::util::container::vec::Vec;
use super::Bus;

/// The port used to specify the configuration address.
const CONFIG_ADDRESS_PORT: u16 = 0xcf8;
/// The port used to retrieve the devices informations.
const CONFIG_DATA_PORT: u16 = 0xcfc;

/// Structure representing a device attached to the PCI bus.
pub struct PCIDevice {
	/// The PCI bus of the device.
	bus: u8,
	/// The offset of the device on the bus.
	device: u8,
	/// The function number of the device.
	function: u8,

	/// The device's ID.
	device_id: u16,
	/// The device's vendor ID.
	vendor_id: u16,

	/// The device's class code, telling what the device is.
	class: u8,
	/// The device's subclass code, giving more informations on the device.
	subclass: u8,
	/// Value giving more informations on the device's compatibilities.
	prog_if: u8,
	/// The device's revision ID.
	revision_id: u8,

	/// Built-In Self Test status.
	bist: u8,
	/// Defines the header type of the device, to determine what informations follow.
	header_type: u8,

	/// Specifies the latency timer in units of PCI bus clocks.
	latency_timer: u8,
	/// Specifies the system cache line size in 32-bit units.
	cache_line_size: u8,

	/// Additional informations about the device.
	info: [u32; 12],
}

impl PCIDevice {
	/// Creates a new instance of PCI device.
	/// `bus` is the PCI bus.
	/// `device` is the device number on the bus.
	/// `function` is the function number on the device.
	/// `data` is the data returned by the PCI.
	fn new(bus: u8, device: u8, function: u8, data: &[u32; 16]) -> Self {
		Self {
			bus,
			device,
			function,

			vendor_id: (data[0] & 0xffff) as _,
			device_id: ((data[0] >> 16) & 0xffff) as _,

			class: ((data[2] >> 24) & 0xff) as _,
			subclass: ((data[2] >> 16) & 0xff) as _,
			prog_if: ((data[2] >> 8) & 0xff) as _,
			revision_id: (data[2] & 0xff) as _,

			bist: ((data[3] >> 24) & 0xff) as _,
			header_type: ((data[3] >> 16) & 0xff) as _,

			latency_timer: ((data[3] >> 8) & 0xff) as _,
			cache_line_size: (data[3] & 0xff) as _,

			info: [
				data[4],
				data[5],
				data[6],
				data[7],
				data[8],
				data[9],
				data[10],
				data[11],
				data[12],
				data[13],
				data[14],
				data[15],
			],
		}
	}

	/// Returns the PCI bus ID.
	#[inline(always)]
	pub fn get_bus(&self) -> u8 {
		self.bus
	}

	/// Returns the PCI device ID.
	#[inline(always)]
	pub fn get_device(&self) -> u8 {
		self.device
	}

	/// Returns the PCI function ID.
	#[inline(always)]
	pub fn get_function(&self) -> u8 {
		self.device
	}

	/// Returns the device ID.
	#[inline(always)]
	pub fn get_device_id(&self) -> u16 {
		self.device_id
	}

	/// Returns the vendor ID.
	#[inline(always)]
	pub fn get_vendor_id(&self) -> u16 {
		self.vendor_id
	}

	/// Returns the class of the device.
	#[inline(always)]
	pub fn get_class(&self) -> u8 {
		self.class
	}

	/// Returns the subclass of the device.
	#[inline(always)]
	pub fn get_subclass(&self) -> u8 {
		self.subclass
	}

	/// Returns the header type of the device.
	#[inline(always)]
	pub fn get_header_type(&self) -> u8 {
		// Clear the Multi-Function flag
		self.header_type & 0b01111111
	}

	/// Returns the `n`'th BAR.
	/// If the BAR doesn't exist, the function returns None.
	pub fn get_bar(&self, n: u8) -> Option<u32> {
		match self.get_header_type() {
			0x00 => {
				if n < 6 {
					Some(self.info[n as usize])
				} else {
					None
				}
			},

			0x01 => {
				if n < 2 {
					Some(self.info[n as usize])
				} else {
					None
				}
			},

			_ => None,
		}
	}

	/// Returns the interrupt PIN used by the device.
	pub fn get_interrupt_pin(&self) -> Option<u8> {
		let n = ((self.info[11] >> 8) & 0xff) as u8;

		if n != 0 {
			Some(n)
		} else {
			None
		}
	}
}

impl PhysicalDevice for PCIDevice {
	fn get_device_id(&self) -> u16 {
		self.device_id
	}

	fn get_vendor_id(&self) -> u16 {
		self.vendor_id
	}

	fn get_class(&self) -> u16 {
		self.class as _
	}

	fn get_subclass(&self) -> u16 {
		self.subclass as _
	}

	fn is_hotplug(&self) -> bool {
		false
	}
}

/// Structure representing the PCI manager.
pub struct PCIManager {}

/// Trait representing a bus.
impl Bus for PCIManager {
	fn get_name(&self) -> &str {
		"PCI"
	}

	fn is_hotplug(&self) -> bool {
		false
	}
}

impl PCIManager {
	// FIXME Currently reading 32 bits?
	/// Reads 16 bits from the PCI register specified by `bus`, `device`, `func` and `off`.
	fn read_word(bus: u8, device: u8, func: u8, off: u8) -> u32 {
		let addr = ((bus as u32) << 16) | ((device as u32) << 11) | ((func as u32) << 8)
			| ((off as u32) & 0xfc) | 0x80000000;
		unsafe {
			io::outl(CONFIG_ADDRESS_PORT, addr);
			io::inl(CONFIG_DATA_PORT)
		}
	}

	/// Reads a device's data and writes it into `data`.
	fn read_data(bus: u8, device: u8, func: u8, data: &mut [u32; 16]) {
		for (i, d) in data.iter_mut().enumerate().skip(1) {
			*d = Self::read_word(bus, device, func, (i * size_of::<u32>()) as _);
		}
	}

	// TODO Cache devices?
	/// Scans for PCI devices and returns the list.
	pub fn scan(&mut self) -> Vec<PCIDevice> {
		let mut devices = Vec::new();

		for bus in 0..=255 {
			for device in 0..32 {
				let first_word = Self::read_word(bus, device, 0, 0);
				let vendor_id = (first_word & 0xffff) as u16;
				// If the device doesn't exist, ignore
				if vendor_id == 0xffff {
					continue;
				}

				// Reading device's PCI data
				let mut data: [u32; 16] = [0; 16];
				Self::read_data(bus, device, 0, &mut data);

				let header_type = ((data[3] >> 16) & 0xff) as u8;
				let max_functions_count = {
					if header_type & 0x80 != 0 {
						// Multi-function device
						8
					} else {
						// Single-function device
						1
					}
				};

				// Iterating on every functions of the device
				for func in 0..max_functions_count {
					let first_word = Self::read_word(bus, device, func, 0);
					let vendor_id = (first_word & 0xffff) as u16;
					// If the function doesn't exist, ignore
					if vendor_id == 0xffff {
						continue;
					}

					// Reading function's PCI data
					Self::read_data(bus, device, 0, &mut data);

					let dev = PCIDevice::new(bus, device, func, &data);
					if devices.push(dev).is_err() {
						crate::kernel_panic!("No enough memory to scan PCI devices!");
					}
				}
			}
		}

		devices
	}
}
