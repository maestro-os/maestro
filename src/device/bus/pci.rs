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
use crate::device::DeviceManager;
use crate::device::bar::BAR;
use crate::device::driver;
use crate::device::manager::PhysicalDevice;
use crate::device::manager;
use crate::errno::Errno;
use crate::io;
use crate::util::container::vec::Vec;

/// The port used to specify the configuration address.
const CONFIG_ADDRESS_PORT: u16 = 0xcf8;
/// The port used to retrieve the devices informations.
const CONFIG_DATA_PORT: u16 = 0xcfc;

/// Device class: Unclassified
pub const CLASS_UNCLASSIFIED: u16 = 0x00;
/// Device class: Mass Storage Controller
pub const CLASS_MASS_STORAGE_CONTROLLER: u16 = 0x01;
/// Device class: Network Controller
pub const CLASS_NETWORK_CONTROLLER: u16 = 0x02;
/// Device class: Display Controller
pub const CLASS_DISPLAY_CONTROLLER: u16 = 0x03;
/// Device class: Multimedia Controller
pub const CLASS_MULTIMEDIA_CONTROLLER: u16 = 0x04;
/// Device class: Memory Controller
pub const CLASS_MEMORY_CONTROLLER: u16 = 0x05;
/// Device class: Bridge
pub const CLASS_BRIDGE: u16 = 0x06;
/// Device class: Simple Communication Controller
pub const CLASS_SIMPLE_COMMUNICATION_CONTROLLER: u16 = 0x07;
/// Device class: Base System Peripheral
pub const CLASS_BASE_SYSTEM_PERIPHERAL: u16 = 0x08;
/// Device class: Input Device Controller
pub const CLASS_INPUT_DEVICE_CONTROLLER: u16 = 0x09;
/// Device class: Docking Station
pub const CLASS_DOCKING_STATION: u16 = 0x0a;
/// Device class: Processor
pub const CLASS_PROCESSOR: u16 = 0x0b;
/// Device class: Serial Bus Controller
pub const CLASS_SERIAL_BUS_CONTROLLER: u16 = 0x0c;
/// Device class: Wireless Controller
pub const CLASS_WIRELESS_CONTROLLER: u16 = 0x0d;
/// Device class: Intelligent Controller
pub const CLASS_INTELLIGENT_CONTROLLER: u16 = 0x0e;
/// Device class: Satellite Communication Controller
pub const CLASS_SATELLITE_COMMUNICATION_CONTROLLER: u16 = 0x0f;
/// Device class: Encryption Controller
pub const CLASS_ENCRYPTION_CONTROLLER: u16 = 0x10;
/// Device class: Signal Processing Controller
pub const CLASS_SIGNAL_PROCESSING_CONTROLLER: u16 = 0x11;
/// Device class: Processing Accelerator
pub const CLASS_PROCESSING_ACCELERATOR: u16 = 0x12;
/// Device class: Non-Essential Instrumentation
pub const CLASS_NON_ESSENTIAL_INSTRUMENTATION: u16 = 0x13;
/// Device class: Co-Processor
pub const CLASS_CO_PROCESSOR: u16 = 0x40;
/// Device class: Unassigned
pub const CLASS_UNASSIGNED: u16 = 0xff;

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

	/// The command register.
	command: u16,
	/// The status register.
	status: u16,

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

			command: (data[1] & 0xffff) as _,
			status: ((data[1] >> 16) & 0xffff) as _,

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

	/// Returns the header type of the device.
	#[inline(always)]
	pub fn get_header_type(&self) -> u8 {
		// Clear the Multi-Function flag
		self.header_type & 0b01111111
	}

	/// Returns the value for the `n`th BAR.
	pub fn get_bar_value(&self, n: u8) -> Option<u32> {
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

	fn get_command_reg(&self) -> Option<u16> {
		Some(self.command)
	}

	fn get_status_reg(&self) -> Option<u16> {
		Some(self.status)
	}

	fn get_class(&self) -> u16 {
		self.class as _
	}

	fn get_subclass(&self) -> u16 {
		self.subclass as _
	}

	fn get_prog_if(&self) -> u8 {
		self.prog_if
	}

	fn is_hotplug(&self) -> bool {
		false
	}

	fn get_bar(&self, n: u8) -> Option<BAR> {
		BAR::from_pci(self, n)
	}
}

/// Reads 16 bits from the PCI register specified by `bus`, `device`, `func` and `off`.
fn read_word(bus: u8, device: u8, func: u8, off: u8) -> u16 {
	// The PCI address
	let addr = ((bus as u32) << 16) | ((device as u32) << 11) | ((func as u32) << 8)
		| ((off as u32) & 0xfc) | 0x80000000;

	let val = unsafe {
		// Setting the address
		io::outl(CONFIG_ADDRESS_PORT, addr);
		// Reading the value
		io::inl(CONFIG_DATA_PORT)
	};
	((val >> ((off & 2) * 8)) & 0xffff) as _
}

/// Reads a device's data and writes it into `data`.
fn read_data(bus: u8, device: u8, func: u8, data: &mut [u32; 16]) {
	for (i, d) in data.iter_mut().enumerate() {
		let v0 = read_word(bus, device, func, ((2 * i) * size_of::<u16>()) as _);
		let v1 = read_word(bus, device, func, ((2 * i + 1) * size_of::<u16>()) as _);

		*d = ((v1 as u32) << 16) | v0 as u32;
	}
}

/// This manager handles every devices connected to the PCI bus. Since the PCI bus is not a hotplug
/// bus, calling on_unplug on this structure has no effect.
pub struct PCIManager {
	/// The list of PCI devices.
	devices: Vec<PCIDevice>,
}

impl PCIManager {
	/// Creates a new instance.
	pub fn new() -> Self {
		Self {
			devices: Vec::new(),
		}
	}

	/// Scans for PCI devices and registers them on the manager.
	/// If the PCI has already been scanned, this function does nothing.
	pub fn scan(&mut self) -> Result<(), Errno> {
		// Avoid calling `on_plug` twice for the same devices
		if !self.devices.is_empty() {
			return Ok(());
		}

		for bus in 0..=255 {
			for device in 0..32 {
				let first_word = read_word(bus, device, 0, 0);
				let vendor_id = (first_word & 0xffff) as u16;
				// If the device doesn't exist, ignore
				if vendor_id == 0xffff {
					continue;
				}

				// Reading device's PCI data
				let mut data: [u32; 16] = [0; 16];
				read_data(bus, device, 0, &mut data);

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
					let first_word = read_word(bus, device, func, 0);
					let vendor_id = (first_word & 0xffff) as u16;
					// If the function doesn't exist, ignore
					if vendor_id == 0xffff {
						continue;
					}

					// Reading function's PCI data
					read_data(bus, device, func, &mut data);

					// Registering the device
					let dev = PCIDevice::new(bus, device, func, &data);
					driver::on_plug(&dev);
					manager::on_plug(&dev);
					self.devices.push(dev)?;
				}
			}
		}

		Ok(())
	}

	/// Returns the list of PCI devices.
	/// If the PCI hasn't been scanned, the function returns an empty vector.
	#[inline(always)]
	pub fn get_devices(&self) -> &Vec<PCIDevice> {
		&self.devices
	}
}

impl DeviceManager for PCIManager {
	fn get_name(&self) -> &str {
		"PCI"
	}

	fn legacy_detect(&mut self) -> Result<(), Errno> {
		Ok(())
	}

	fn on_plug(&mut self, _dev: &dyn PhysicalDevice) {}

	fn on_unplug(&mut self, _dev: &dyn PhysicalDevice) {}
}
