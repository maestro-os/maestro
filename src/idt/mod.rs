pub mod pic;

use crate::memory::Void;
use crate::util;

/*
 * TODO Doc
 */
const ID_TYPE_GATE_TASK: u8 = 0b01010000;
/*
 * TODO Doc
 */
const ID_TYPE_GATE_INTERRUPT16: u8 = 0b01100000;
/*
 * TODO Doc
 */
const ID_TYPE_GATE_TRAP16: u8 = 0b01110000;
/*
 * TODO Doc
 */
const ID_TYPE_GATE_INTERRUPT32: u8 = 0b11100000;
/*
 * TODO Doc
 */
const ID_TYPE_GATE_TRAP32: u8 = 0b11110000;
/*
 * TODO Doc
 */
const ID_TYPE_S: u8 = 0b00001000;
/*
 * TODO Doc
 */
const ID_PRIVILEGE_RING_0: u8 = 0b00000000;
/*
 * TODO Doc
 */
const ID_PRIVILEGE_RING_1: u8 = 0b00000010;
/*
 * TODO Doc
 */
const ID_PRIVILEGE_RING_2: u8 = 0b00000100;
/*
 * TODO Doc
 */
const ID_PRIVILEGE_RING_3: u8 = 0b00000110;
/*
 * TODO Doc
 */
const ID_PRESENT: u8 = 0b00000001;

/*
 * Disables interruptions.
 */
#[macro_export]
macro_rules! cli {
	() => (unsafe { asm!("cli") });
}

/*
 * Enables interruptions.
 */
#[macro_export]
macro_rules! sti {
	() => (unsafe { asm!("sti") });
}

/*
 * Waits for an interruption.
 */
#[macro_export]
macro_rules! hlt {
	() => (unsafe { asm!("hlt") });
}

/*
 * The IDT vector index for system calls.
 */
const SYSCALL_VECTOR: u8 = 0x80;
/*
 * The number of entries into the IDT.
 */
const ENTRIES_COUNT: u8 = 0x81;

/*
 * Structure representing the IDT.
 */
#[repr(C, packed)]
struct InterruptDescriptorTable {
	/* TODO doc */
	size: u16,
	/* TODO doc */
	offset: u32,
}

/*
 * Structure representing an IDT entry.
 */
#[repr(C)]
struct InterruptDescriptor {
	/* TODO doc */
	offset: u16,
	/* TODO doc */
	selector: u16,
	/* TODO doc */
	zero: u8,
	/* TODO doc */
	type_attr: u8,
	/* TODO doc */
	offset_2: u16,
}

extern "C" {
	fn idt_load(idt: *const Void);
	fn interrupt_is_enabled() -> bool;
}

extern "C" {
	fn irq0();
	fn irq1();
	fn irq2();
	fn irq3();
	fn irq4();
	fn irq5();
	fn irq6();
	fn irq7();
	fn irq8();
	fn irq9();
	fn irq10();
	fn irq11();
	fn irq12();
	fn irq13();
	fn irq14();
	fn irq15();

	fn error0();
	fn error1();
	fn error2();
	fn error3();
	fn error4();
	fn error5();
	fn error6();
	fn error7();
	fn error8();
	fn error9();
	fn error10();
	fn error11();
	fn error12();
	fn error13();
	fn error14();
	fn error15();
	fn error16();
	fn error17();
	fn error18();
	fn error19();
	fn error20();
	fn error21();
	fn error22();
	fn error23();
	fn error24();
	fn error25();
	fn error26();
	fn error27();
	fn error28();
	fn error29();
	fn error30();
	fn error31();

	fn syscall();
}

/*
 * The list of IDT entries.
 */
static mut ID: [InterruptDescriptor; ENTRIES_COUNT as usize] = [InterruptDescriptor {
	offset: 0,
	selector: 0,
	zero: 0,
	type_attr: 0,
	offset_2: 0,
}; 0x81];

/*
 * Creates an IDT entry.
 */
fn create_id(address: *const Void, selector: u16, type_attr: u8) -> InterruptDescriptor {
	InterruptDescriptor {
		offset: ((address as u32) & 0xffff) as u16,
		selector: selector,
		zero: 0,
		type_attr: type_attr,
		offset_2: (((address as u32) & 0xffff0000) >> util::bit_size_of::<u16>()) as u16,
	}
}

/*
 * Takes a C extern function and returns its pointer.
 */
fn get_c_fn_ptr(f: unsafe extern "C" fn()) -> *const Void {
	unsafe {
		core::mem::transmute::<_, _>(f as *const Void)
	}
}

/*
 * Initializes the IDT.
 */
pub fn init() {
	cli!();
	pic::init(0x20, 0x28);

	unsafe {
		ID[0x00] = create_id(get_c_fn_ptr(error0), 0x8, 0x8e);
		ID[0x01] = create_id(get_c_fn_ptr(error1), 0x8, 0x8e);
		ID[0x02] = create_id(get_c_fn_ptr(error2), 0x8, 0x8e);
		ID[0x03] = create_id(get_c_fn_ptr(error3), 0x8, 0x8e);
		ID[0x04] = create_id(get_c_fn_ptr(error4), 0x8, 0x8e);
		ID[0x05] = create_id(get_c_fn_ptr(error5), 0x8, 0x8e);
		ID[0x06] = create_id(get_c_fn_ptr(error6), 0x8, 0x8e);
		ID[0x07] = create_id(get_c_fn_ptr(error7), 0x8, 0x8e);
		ID[0x08] = create_id(get_c_fn_ptr(error8), 0x8, 0x8e);
		ID[0x09] = create_id(get_c_fn_ptr(error9), 0x8, 0x8e);
		ID[0x0a] = create_id(get_c_fn_ptr(error10), 0x8, 0x8e);
		ID[0x0b] = create_id(get_c_fn_ptr(error11), 0x8, 0x8e);
		ID[0x0c] = create_id(get_c_fn_ptr(error12), 0x8, 0x8e);
		ID[0x0d] = create_id(get_c_fn_ptr(error13), 0x8, 0x8e);
		ID[0x0e] = create_id(get_c_fn_ptr(error14), 0x8, 0x8e);
		ID[0x0f] = create_id(get_c_fn_ptr(error15), 0x8, 0x8e);
		ID[0x10] = create_id(get_c_fn_ptr(error16), 0x8, 0x8e);
		ID[0x11] = create_id(get_c_fn_ptr(error17), 0x8, 0x8e);
		ID[0x12] = create_id(get_c_fn_ptr(error18), 0x8, 0x8e);
		ID[0x13] = create_id(get_c_fn_ptr(error19), 0x8, 0x8e);
		ID[0x14] = create_id(get_c_fn_ptr(error20), 0x8, 0x8e);
		ID[0x15] = create_id(get_c_fn_ptr(error21), 0x8, 0x8e);
		ID[0x16] = create_id(get_c_fn_ptr(error22), 0x8, 0x8e);
		ID[0x17] = create_id(get_c_fn_ptr(error23), 0x8, 0x8e);
		ID[0x18] = create_id(get_c_fn_ptr(error24), 0x8, 0x8e);
		ID[0x19] = create_id(get_c_fn_ptr(error25), 0x8, 0x8e);
		ID[0x1a] = create_id(get_c_fn_ptr(error26), 0x8, 0x8e);
		ID[0x1b] = create_id(get_c_fn_ptr(error27), 0x8, 0x8e);
		ID[0x1c] = create_id(get_c_fn_ptr(error28), 0x8, 0x8e);
		ID[0x1d] = create_id(get_c_fn_ptr(error29), 0x8, 0x8e);
		ID[0x1e] = create_id(get_c_fn_ptr(error30), 0x8, 0x8e);
		ID[0x1f] = create_id(get_c_fn_ptr(error31), 0x8, 0x8e);

		ID[0x20] = create_id(get_c_fn_ptr(irq0), 0x8, 0x8e);
		ID[0x21] = create_id(get_c_fn_ptr(irq1), 0x8, 0x8e);
		ID[0x22] = create_id(get_c_fn_ptr(irq2), 0x8, 0x8e);
		ID[0x23] = create_id(get_c_fn_ptr(irq3), 0x8, 0x8e);
		ID[0x24] = create_id(get_c_fn_ptr(irq4), 0x8, 0x8e);
		ID[0x25] = create_id(get_c_fn_ptr(irq5), 0x8, 0x8e);
		ID[0x26] = create_id(get_c_fn_ptr(irq6), 0x8, 0x8e);
		ID[0x27] = create_id(get_c_fn_ptr(irq7), 0x8, 0x8e);
		ID[0x28] = create_id(get_c_fn_ptr(irq8), 0x8, 0x8e);
		ID[0x29] = create_id(get_c_fn_ptr(irq9), 0x8, 0x8e);
		ID[0x2a] = create_id(get_c_fn_ptr(irq10), 0x8, 0x8e);
		ID[0x2b] = create_id(get_c_fn_ptr(irq11), 0x8, 0x8e);
		ID[0x2c] = create_id(get_c_fn_ptr(irq12), 0x8, 0x8e);
		ID[0x2d] = create_id(get_c_fn_ptr(irq13), 0x8, 0x8e);
		ID[0x2e] = create_id(get_c_fn_ptr(irq14), 0x8, 0x8e);
		ID[0x2f] = create_id(get_c_fn_ptr(irq15), 0x8, 0x8e);

		ID[SYSCALL_VECTOR as usize] = create_id(get_c_fn_ptr(syscall), 0x8, 0xee);
	}

	let idt = InterruptDescriptorTable {
		size: (core::mem::size_of::<InterruptDescriptor>() * (ENTRIES_COUNT as usize) - 1) as u16,
		offset: unsafe { &ID } as *const _ as u32,
	};
	unsafe {
		idt_load(&idt as *const _ as *const _);
	}
}
