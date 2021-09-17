//! The PCI (Peripheral Component Interconnect) is a bus which allows to attach hardware devices on
//! the motherboard. There here-module allows to retrieve informations on the devices attached to
//! the computer's pCI.

use crate::device::manager::PhysicalDevice;
use crate::io;
use crate::util::container::vec::Vec;
use super::Bus;

/// The port used to specify the configuration address.
const CONFIG_ADDRESS_PORT: u16 = 0xcf8;
/// The port used to retrieve the devices informations.
const CONFIG_DATA_PORT: u16 = 0xcfc;

pub enum PCIDeviceInfo {
	/// A casual PCI device.
	Device(),
	/// PCI-to-PCI bridge.
	PCIBridge(),
	/// PCI-to-CardBus bridge.
	CardBusBridge(),
}

/// Structure representing a device attached to the PCI bus.
pub struct PCIDevice {
	/// The PCI bus of the device.
	bus: u8,
	/// The offset of the device on the bus.
	device: u8,

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
	/// Checks if a device exists on the given bus `bus` and device id `device` and returns an
	/// instance for it if so.
	/// If no device is present at this location, the function returns None.
	fn new(manager: &mut PCIManager, bus: u8, device: u8) -> Option<Self> {
		let first_word = manager.read_word(bus, device, 0, 0);
		let vendor_id = (first_word & 0xffff) as u16;
		if vendor_id != 0xffff {
			let device_id = ((first_word >> 16) & 0xffff) as u16;
			let mut data: [u32; 16] = [0; 16];
			data[0] = first_word;
			for (i, d) in data.iter_mut().enumerate().skip(1) {
				*d = manager.read_word(bus, device, 0, (i * 4) as _);
			}

			Some(Self {
				bus,
				device,

				vendor_id,
				device_id,

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
			})
		} else {
			None
		}
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

	/// Returns the `n`'th BAR.
	/// If the BAR doesn't exist, the function returns None.
	pub fn get_BAR(&self, n: u8) -> Option<u32> {
		match self.header_type {
			0x00 => {
				if n < 6 {
					self.info[n]
				} else {
					None
				}
			},

			0x01 => {
				if n < 2 {
					self.info[n]
				} else {
					None
				}
			},

			None,
		}
	}
}

impl PhysicalDevice for PCIDevice {
	fn get_product_id(&self) -> u16 {
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
	fn read_word(&self, bus: u8, device: u8, func: u8, off: u8) -> u32 {
		let addr = ((bus as u32) << 16) | ((device as u32) << 11) | ((func as u32) << 8)
			| ((off as u32) & 0xfc) | 0x80000000;
		unsafe {
			io::outl(CONFIG_ADDRESS_PORT, addr);
			io::inl(CONFIG_DATA_PORT)
		}
	}

	// TODO Cache devices?
	/// Scans for PCI devices and returns the list.
	pub fn scan(&mut self) -> Vec<PCIDevice> {
		let mut devices = Vec::new();

		for bus in 0..=255 {
			for device in 0..32 {
				if let Some(dev) = PCIDevice::new(self, bus, device) {
					if devices.push(dev).is_err() {
						crate::kernel_panic!("No enough memory to scan PCI devices!");
					}
				} else {
					break;
				}
			}
		}

		devices
	}
}
