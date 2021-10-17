//! Maestro is a Unix kernel written in Rust. This reference documents interfaces for modules and
//! the kernel's internals.

#![no_std]

#![allow(unused_attributes)]
#![no_main]

#![feature(allow_internal_unstable)]
#![feature(asm)]
#![feature(coerce_unsized)]
#![feature(const_fn_trait_bound)]
#![feature(const_maybe_uninit_assume_init)]
#![feature(const_mut_refs)]
#![feature(const_ptr_offset)]
#![feature(const_raw_ptr_deref)]
#![feature(core_intrinsics)]
#![feature(custom_test_frameworks)]
#![feature(dispatch_from_dyn)]
#![feature(fundamental)]
#![feature(lang_items)]
#![feature(llvm_asm)]
#![feature(maybe_uninit_extra)]
#![feature(panic_info_message)]
#![feature(slice_ptr_get)]
#![feature(stmt_expr_attributes)]
#![feature(unsize)]

#![deny(warnings)]
#![allow(dead_code)]
#![allow(unused_macros)]

#![test_runner(crate::selftest::runner)]
#![reexport_test_harness_main = "kernel_selftest"]

pub mod acpi;
pub mod cmdline;
pub mod debug;
pub mod device;
pub mod elf;
pub mod errno;
pub mod event;
pub mod file;
pub mod gdt;
#[macro_use]
pub mod idt;
pub mod io;
pub mod limits;
pub mod logger;
pub mod memory;
pub mod module;
pub mod multiboot;
#[macro_use]
pub mod panic;
pub mod pit;
#[macro_use]
pub mod print;
pub mod process;
pub mod selftest;
pub mod syscall;
pub mod time;
pub mod tty;
#[macro_use]
pub mod util;
#[macro_use]
pub mod vga;

use core::ffi::c_void;
use core::mem::MaybeUninit;
use core::panic::PanicInfo;
use core::ptr::null;
use crate::errno::Errno;
use crate::file::path::Path;
use crate::memory::vmem::VMem;
use crate::memory::vmem;
use crate::process::Process;
use crate::util::boxed::Box;
use crate::util::lock::mutex::Mutex;

/// The kernel's name.
pub const NAME: &str = "maestro";
/// Current kernel version.
pub const VERSION: &str = "1.0";

/// The path to the init process binary.
const INIT_PATH: &str = "/sbin/init";

extern "C" {
	fn kernel_wait();
	fn kernel_loop() -> !;
	fn kernel_loop_reset(stack: *mut c_void) -> !;
	fn kernel_halt() -> !;
}

/// Makes the kernel wait for an interrupt, then returns.
/// This function enables interrupts.
pub fn wait() {
	unsafe {
		kernel_wait();
	}
}

/// Enters the kernel loop and processes every interrupts indefinitely.
pub fn enter_loop() -> ! {
	unsafe {
		kernel_loop();
	}
}

/// Resets the stack to the given value, then calls `enter_loop`.
/// The function is unsafe because the pointer passed in parameter might be invalid.
pub unsafe fn loop_reset(stack: *mut c_void) -> ! {
	kernel_loop_reset(stack);
}

/// Halts the kernel until reboot.
pub fn halt() -> ! {
	unsafe {
		kernel_halt();
	}
}

/// Field storing the kernel's virtual memory context.
static mut KERNEL_VMEM: MaybeUninit<Mutex<Box<dyn VMem>>> = MaybeUninit::uninit();

/// Initializes the kernel's virtual memory context.
fn init_vmem() -> Result<(), Errno> {
	let mut kernel_vmem = vmem::new()?;

	// TODO If Meltdown mitigation is enabled, only allow read access to a stub of the
	// kernel for interrupts

	// TODO Enable GLOBAL in cr4

	// Mapping the kernelspace
	kernel_vmem.map_range(null::<c_void>(),
		memory::PROCESS_END,
		memory::get_kernelspace_size() / memory::PAGE_SIZE,
		vmem::x86::FLAG_WRITE | vmem::x86::FLAG_GLOBAL)?;

	// Mapping VGA's buffer
	let vga_flags = vmem::x86::FLAG_CACHE_DISABLE | vmem::x86::FLAG_WRITE_THROUGH
		| vmem::x86::FLAG_WRITE;
	kernel_vmem.map_range(vga::BUFFER_PHYS as _, vga::get_buffer_virt() as _, 1, vga_flags)?;

	// Making the kernel image read-only
	kernel_vmem.protect_kernel()?;

	unsafe {
		*KERNEL_VMEM.assume_init_mut() = Mutex::new(kernel_vmem);
	}

	// Binding the kernel virtual memory context
	bind_vmem();
	Ok(())
}

/// Returns the kernel's virtual memory context.
pub fn get_vmem() -> &'static mut Mutex<Box<dyn VMem>> {
	unsafe { // Safe because using Mutex
		KERNEL_VMEM.assume_init_mut()
	}
}

/// Binds the kernel's virtual memory context.
pub fn bind_vmem() {
	let guard = unsafe { // Safe because using Mutex
		KERNEL_VMEM.assume_init_mut()
	}.lock(false);
	guard.get().bind();
}

extern "C" {
	fn test_process();
}

/// Launches the init process.
fn init() -> Result<(), &'static str> {
	// TODO Add a compilation option to enable test process
	//let test_begin = unsafe {
	//	core::mem::transmute::<unsafe extern "C" fn(), *const c_void>(test_process)
	//};
	//if Process::new_init(test_begin).is_err() {
	//	kernel_panic!("Failed to create init process!", 0);
	//}

	let mut mutex = Process::new().or(Err("Failed to create init process!"))?;
	let mut lock = mutex.lock(false);
	let proc = lock.get_mut();

	let path = Path::from_string(INIT_PATH, false).or(Err("Unknown error"))?;

	// TODO Add default env
	let result = proc.exec(&path, &[INIT_PATH], &[]);
	if result.is_ok() {
		return Ok(());
	}

	let err_str = match result.unwrap_err() {
		errno::ENOENT => "Cannot file init process binary!",
		errno::ENOMEM => "Not enough memory to run the init process!",

		_ => "Unknown error",
	};
	Err(err_str)
}

/// This is the main function of the Rust source code, responsible for the initialization of the
/// kernel. When calling this function, the CPU must be in Protected Mode with the GDT loaded with
/// space for the Task State Segment.
/// `magic` is the magic number passed by Multiboot.
/// `multiboot_ptr` is the pointer to the Multiboot booting informations structure.
#[no_mangle]
pub extern "C" fn kernel_main(magic: u32, multiboot_ptr: *const c_void) -> ! {
	crate::cli!();
	tty::init();

	if magic != multiboot::BOOTLOADER_MAGIC || !util::is_aligned(multiboot_ptr, 8) {
		kernel_panic!("Bootloader non compliant with Multiboot2!", 0);
	}

	idt::init();
	pit::init();
	event::init();

	// TODO CPUID
	multiboot::read_tags(multiboot_ptr);

	memory::memmap::init(multiboot_ptr);
	if cfg!(config_debug_debug) {
		memory::memmap::print_entries();
	}
	memory::alloc::init();
	memory::malloc::init();

	if init_vmem().is_err() {
		crate::kernel_panic!("Cannot initialize kernel virtual memory!", 0);
	}

	#[cfg(test)]
	#[cfg(config_debug_test)]
	kernel_selftest();

	let args_parser = cmdline::ArgsParser::parse(&multiboot::get_boot_info().cmdline);
	if let Err(e) = args_parser {
		e.print();
		crate::halt();
	}
	let args_parser = args_parser.unwrap();
	logger::init(args_parser.is_silent());

	println!("Booting Maestro kernel version {}", crate::VERSION);

	println!("Initializing ACPI...");
	acpi::init();

	println!("Initializing ramdisks...");
	if device::storage::ramdisk::create().is_err() {
		kernel_panic!("Failed to create ramdisks!");
	}
	println!("Initializing devices management...");
	if device::init().is_err() {
		crate::kernel_panic!("Failed to initialize devices management!", 0);
	}

	let (root_major, root_minor) = args_parser.get_root_dev();
	println!("Root device is {} {}", root_major, root_minor);
	println!("Initializing files management...");
	if file::init(device::DeviceType::Block, root_major, root_minor).is_err() {
		kernel_panic!("Failed to initialize files management!");
	}
	if device::default::create().is_err() {
		kernel_panic!("Failed to create default devices!");
	}

	println!("Initializing processes...");
	if process::init().is_err() {
		kernel_panic!("Failed to init processes!", 0);
	}

	if let Err(e) = init() {
		kernel_panic!(e, 0);
	}
	crate::enter_loop();
}

/// Called on Rust panic.
#[panic_handler]
fn panic(panic_info: &PanicInfo) -> ! {
	#[cfg(test)]
	if selftest::is_running() {
		println!("FAILED\n");
		println!("Error: {}\n", panic_info);

		#[cfg(config_debug_qemu)]
		selftest::qemu::exit(selftest::qemu::FAILURE);
		#[cfg(not(config_debug_qemu))]
		halt();
	}

	if let Some(s) = panic_info.message() {
		panic::rust_panic(s);
	} else {
		crate::kernel_panic!("Rust panic (no payload)", 0);
	}
}

/// Function that is required to be implemented by the Rust compiler and is used only when
/// panicking.
#[lang = "eh_personality"]
fn eh_personality() {}
