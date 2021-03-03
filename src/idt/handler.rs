/// TODO doc

use crate::idt::pic;

#[no_mangle]
pub extern "C" fn irq1_handler() {
	// TODO Keyboard
	pic::end_of_interrupt(0x1);
}

#[no_mangle]
pub extern "C" fn irq2_handler() {
	// TODO
	pic::end_of_interrupt(0x2);
}

#[no_mangle]
pub extern "C" fn irq3_handler() {
	// TODO
	pic::end_of_interrupt(0x3);
}

#[no_mangle]
pub extern "C" fn irq4_handler() {
	// TODO
	pic::end_of_interrupt(0x4);
}

#[no_mangle]
pub extern "C" fn irq5_handler() {
	// TODO
	pic::end_of_interrupt(0x5);
}

#[no_mangle]
pub extern "C" fn irq6_handler() {
	// TODO
	pic::end_of_interrupt(0x6);
}

#[no_mangle]
pub extern "C" fn irq7_handler() {
	// TODO
	pic::end_of_interrupt(0x7);
}

#[no_mangle]
pub extern "C" fn irq8_handler() {
	// TODO
	pic::end_of_interrupt(0x8);
}

#[no_mangle]
pub extern "C" fn irq9_handler() {
	// TODO
	pic::end_of_interrupt(0x9);
}

#[no_mangle]
pub extern "C" fn irq10_handler() {
	// TODO
	pic::end_of_interrupt(0xa);
}

#[no_mangle]
pub extern "C" fn irq11_handler() {
	// TODO
	pic::end_of_interrupt(0xb);
}

#[no_mangle]
pub extern "C" fn irq12_handler() {
	// TODO
	pic::end_of_interrupt(0xc);
}

#[no_mangle]
pub extern "C" fn irq13_handler() {
	// TODO
	pic::end_of_interrupt(0xd);
}

#[no_mangle]
pub extern "C" fn irq14_handler() {
	// TODO ATA
	pic::end_of_interrupt(0xe);
}

#[no_mangle]
pub extern "C" fn irq15_handler() {
	// TODO
	pic::end_of_interrupt(0xf);
}
