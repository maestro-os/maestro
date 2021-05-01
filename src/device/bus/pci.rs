/// The PCI (Peripheral Component Interconnect) is a bus which allows to attach hardware devices on
/// the motherboard. There here-module allows to retrieve informations on the devices attached to
/// the computer's pCI.

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

	/// The device's ID.
	device_id: u16,
	/// The device's vendor ID.
	vendor_id: u16,

	/// TODO doc
	status: u16,
	/// TODO doc
	command: u16,

	/// The device's class code, telling what the device is.
	class: u8,
	/// The device's subclass code, giving more informations on the device.
	subclass: u8,
	/// Value giving more informations on the device's compatibilities.
	prog_if: u8,
	/// TODO doc
	revision_id: u8,

	/// TODO doc
	bist: u8,
	/// Defines the header type of the device, to determine what informations follow.
	header_type: u8,

	// TODO Fill additional informations
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
			for i in 1..data.len() {
				data[i] = manager.read_word(bus, device, 0, (i * 4) as _);
			}

			Some(Self {
				bus: bus,
				device: device,

				vendor_id: vendor_id,
				device_id: device_id,

				status: ((data[1] >> 16) & 0xffff) as _,
				command: (data[1] & 0xffff) as _,

				class: ((data[2] >> 24) & 0xff) as _,
				subclass: ((data[2] >> 16) & 0xff) as _,
				prog_if: ((data[2] >> 8) & 0xff) as _,
				revision_id: (data[2] & 0xff) as _,

				bist: ((data[3] >> 24) & 0xff) as _,
				header_type: ((data[3] >> 16) & 0xff) as _,

				// TODO Fill additional informations
			})
		} else {
			None
		}
	}

	/// Returns the device ID.
	pub fn get_device_id(&self) -> u16 {
		self.device_id
	}

	/// Returns the vendor ID.
	pub fn get_vendor_id(&self) -> u16 {
		self.vendor_id
	}

	/// Returns the class of the device.
	pub fn get_class(&self) -> u8 {
		self.class
	}

	/// Returns the subclass of the device.
	pub fn get_subclass(&self) -> u8 {
		self.subclass
	}

	// TODO
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
	pub fn scan(&mut self) -> Vec::<PCIDevice> {
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
