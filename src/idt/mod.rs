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
