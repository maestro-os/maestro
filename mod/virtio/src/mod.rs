//! virtio device drivers

#![cfg_attr(not(maestro_builtin), no_std)]
#![cfg_attr(not(maestro_builtin), no_main)]

#[cfg(not(maestro_builtin))]
#[no_link]
extern crate kernel;

mod blkdev;

use blkdev::VirtioBlkDevOps;
use core::todo;
use kernel::{
	device,
	device::manager::{DeviceManager, PhysicalDevice},
	println,
	utils::{
		boxed::Box,
		collections::vec::Vec,
		errno::{AllocResult, EResult},
	},
};

kernel::module!([]);

const VIRTIO_VENDOR_ID: u16 = 0x1af4;
const VIRTIO_DEVICE_ID_START: u16 = 0x1000;
const VIRTIO_DEVICE_ID_END: u16 = 0x1080;

const NON_TRANSITIONAL_NET_DEV: u16 = 1;
const NON_TRANSITIONAL_BLK_DEV: u16 = 2;
const NON_TRANSITIONAL_CONSOLE: u16 = 3;
const NON_TRANSITIONAL_ENTROPY_SOURCE: u16 = 4;
const NON_TRANSITIONAL_TRAD_MEM_BALLOONING: u16 = 5;

const TRANSITIONAL_NET_DEV: u16 = 0x1000;
const TRANSITIONAL_BLK_DEV: u16 = 0x1001;
const TRANSITIONAL_MEM_BALLOONING: u16 = 0x1002;
const TRANSITIONAL_CONSOLE: u16 = 0x1003;
const TRANSITIONAL_SCSI_HOST: u16 = 0x1004;
const TRANSITIONAL_ENTROPY_SOURCE: u16 = 0x1005;
const TRANSITIONAL_9P_TRANSPORT: u16 = 0x1009;

/// Little-endian unsigned 16 bits
#[allow(non_camel_case_types)]
pub type le16 = u16;
/// Little-endian unsigned 32 bits
#[allow(non_camel_case_types)]
pub type le32 = u32;
/// Little-endian unsigned 64 bits
#[allow(non_camel_case_types)]
pub type le64 = u64;

fn is_virtio_dev(vendor_id: u16, device_id: u16) -> bool {
	if vendor_id != VIRTIO_VENDOR_ID {
		return false;
	}
	if !(VIRTIO_DEVICE_ID_START..VIRTIO_DEVICE_ID_END).contains(&device_id) {
		return false;
	}
	true
}

trait VirtioDeviceOps {
	/// Device-specific setup
	fn setup(&self, dev: &dyn PhysicalDevice) -> EResult<()>;
}

struct VirtioDevice {
	ops: Box<dyn VirtioDeviceOps>,
}

impl VirtioDevice {
	fn new(dev: &dyn PhysicalDevice) -> AllocResult<Option<Self>> {
		let pci_device_id = dev.get_device_id();
		if !is_virtio_dev(dev.get_vendor_id(), pci_device_id) {
			return Ok(None);
		}
		let virtio_device_id = pci_device_id.checked_sub(0x1040);
		let ops = if let Some(virtio_device_id) = virtio_device_id {
			// Non-transitional device
			match virtio_device_id {
				NON_TRANSITIONAL_NET_DEV => todo!(), // TODO add support
				NON_TRANSITIONAL_BLK_DEV => Box::new(VirtioBlkDevOps)?,
				NON_TRANSITIONAL_CONSOLE => todo!(), // TODO add support
				NON_TRANSITIONAL_ENTROPY_SOURCE => todo!(), // TODO add support
				NON_TRANSITIONAL_TRAD_MEM_BALLOONING => todo!(), // TODO add support
				_ => return Ok(None),
			}
		} else {
			// Transitional device
			match pci_device_id {
				// TODO
				_ => return Ok(None),
			}
		};
		// TODO:
		// - reset the device
		// - set the acknowledge status bit
		// - set the driver status bit
		// - read feature bits and write the supported subset back
		// - Set FEATURES_OK status bit
		// - Re-read device status to ensure FEATURES_OK is still set. If not, abort
		let res = ops.setup(dev);
		if let Err(e) = res {
			// TODO set FAILED status bit
			return Err(e);
		}
		// TODO: set DRIVER_OK
		Ok(Some(VirtioDevice {
			ops,
		}))
	}
}

struct VirtioDeviceManager {
	devices: Vec<VirtioDevice>,
}

impl DeviceManager for VirtioDeviceManager {
	fn on_plug(&mut self, dev: &dyn PhysicalDevice) -> EResult<()> {
		let dev = VirtioDevice::new(dev)?;
		if let Some(dev) = dev {
			self.devices.push(dev)?;
		}
		Ok(())
	}

	fn on_unplug(&mut self, dev: &dyn PhysicalDevice) -> EResult<()> {
		let pci_device_id = dev.get_device_id();
		if !is_virtio_dev(dev.get_vendor_id(), pci_device_id) {
			return Ok(());
		}
		todo!() // remove device
	}
}

fn init_impl() -> AllocResult<()> {
	println!("Initialize virtio driver");
	device::manager::register(VirtioDeviceManager {
		devices: Vec::new(),
	})?;
	// TODO rescan PCI
	Ok(())
}

/// Called on module load
#[cfg_attr(not(maestro_builtin), unsafe(no_mangle))]
pub extern "C" fn init() -> bool {
	let res = init_impl();
	if let Err(e) = res {
		println!("Could not initialize virtio driver: {e}");
	}
	res.is_ok()
}

/// Called on module unload
#[cfg_attr(not(maestro_builtin), unsafe(no_mangle))]
pub extern "C" fn fini() {
	println!("Unload virtio driver");
	// TODO unregister manager
}
