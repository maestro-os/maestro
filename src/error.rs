use crate::util;

/*
 * This function is called whenever an error interruption is triggered.
 * TODO doc
 */
pub fn error_handler(error: u32, error_code: u32, _regs: *const util::Regs) {
	// TODO
	::println!("Error: {} {}", error, error_code);
}
