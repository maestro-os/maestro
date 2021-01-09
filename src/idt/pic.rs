use crate::io;

/// TODO doc
const MASTER_COMMAND: u16 = 0x20;
/// TODO doc
const MASTER_DATA: u16 = 0x21;
/// TODO doc
const SLAVE_COMMAND: u16 = 0xa0;
/// TODO doc
const SLAVE_DATA: u16 = 0xa1;

/// TODO doc
const ICW1_ICW4: u8 = 0x01;
/// TODO doc
const ICW1_SINGLE: u8 = 0x02;
/// TODO doc
const ICW1_INTERVAL4: u8 = 0x04;
/// TODO doc
const ICW1_LEVEL: u8 = 0x08;
/// TODO doc
const ICW1_INIT: u8 = 0x10;

/// TODO doc
const ICW3_SLAVE_PIC: u8 = 0x04;
/// TODO doc
const ICW3_CASCADE: u8 = 0x02;

/// TODO doc
const ICW4_8086: u8 = 0x01;
/// TODO doc
const ICW4_AUTO: u8 = 0x02;
/// TODO doc
const ICW4_BUF_SLAVE: u8 = 0x08;
/// TODO doc
const ICW4_BUF_MASTER: u8 = 0x0C;
/// TODO doc
const ICW4_SFNM: u8 = 0x10;

/// TODO doc
const COMMAND_EOI: u8 = 0x20;

/// Initializes the PIC.
pub fn init(offset1: u8, offset2: u8) {
	unsafe {
		let mask1 = io::inb(MASTER_DATA);
		let mask2 = io::inb(SLAVE_DATA);

		io::outb(MASTER_COMMAND, ICW1_INIT | ICW1_ICW4);
		io::outb(SLAVE_COMMAND, ICW1_INIT | ICW1_ICW4);

		io::outb(MASTER_DATA, offset1);
		io::outb(SLAVE_DATA, offset2);

		io::outb(MASTER_DATA, ICW3_SLAVE_PIC);
		io::outb(SLAVE_DATA, ICW3_CASCADE);

		io::outb(MASTER_DATA, ICW4_8086);
		io::outb(SLAVE_DATA, ICW4_8086);

		io::outb(MASTER_DATA, mask1);
		io::outb(SLAVE_DATA, mask2);
	}
}

/// Sends an End-Of-Interrupt message to the PIC for the given interrupt `irq`.
#[no_mangle]
pub extern "C" fn end_of_interrupt(irq: u8) {
	if irq >= 0x8 {
		unsafe {
			io::outb(SLAVE_COMMAND, COMMAND_EOI);
		}
	}
	unsafe {
		io::outb(MASTER_COMMAND, COMMAND_EOI);
	}
}
