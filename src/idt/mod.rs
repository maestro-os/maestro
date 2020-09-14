pub mod pic;

use crate::memory::Void;

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
 * Structure representing an IDT entry.
 */
#[repr(C)]
struct InterruptDescriptor
{
	offset: u16,
	selector: u16,
	zero: u8,
	type_attr: u8,
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
static ID: [InterruptDescriptor; 0x81] = [InterruptDescriptor {
	offset: 0,
	selector: 0,
	zero: 0,
	type_attr: 0,
	offset_2: 0,
}; 0x81];

/*
 * Initializes the IDT.
 */
pub fn init() {
	cli!();
	pic::init(0x20, 0x28);

	// TODO Fill IDT
	// TODO Load IDT
}
