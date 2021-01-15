/// This file handles macros `print` and `println`.

use core::ffi::c_void;

#[cfg(userspace)]
extern "C" {
	//fn write(fd: i32, buf: *const c_void, size: usize) -> isize;
	fn printf(format: *const u8, ...) -> i32;
}

/// Custom writer used to redirect print/println macros to the desired text output.
#[cfg(userspace)]
struct StdOutWrite {}

#[cfg(userspace)]
impl core::fmt::Write for StdOutWrite {
	fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error> {
		unsafe { // Call to C function
			//write(1, s.as_bytes() as *const _ as _, s.len());
			printf(b"bleh" as _);
		}
		Ok(())
	}
}

/// Prints the specified message on the current TTY. This function is meant to be used through
/// `print!` and `println!` macros only.
#[cfg(userspace)]
pub fn _print_stdout(args: core::fmt::Arguments) {
	let mut w: StdOutWrite = StdOutWrite {};
	core::fmt::write(&mut w, args).ok();
}

/// Prints the given formatted string with the given values.
#[cfg(not(userspace))]
#[allow_internal_unstable(print_internals)]
#[macro_export]
macro_rules! print {
	($($arg:tt)*) => {{
		crate::print_util::_print(format_args!($($arg)*));
	}};
}

/// Prints the given formatted string with the given values.
#[cfg(userspace)]
#[allow_internal_unstable(print_internals)]
#[macro_export]
macro_rules! print {
	($($arg:tt)*) => {{
		crate::print::_print_stdout(format_args!($($arg)*));
	}};
}

/// Same as `print!`, except it appends a newline at the end.
#[cfg(not(userspace))]
#[allow_internal_unstable(print_internals, format_args_nl)]
#[macro_export]
macro_rules! println {
	() => (crate::print!("\n"));
	($($arg:tt)*) => {{
		crate::print_util::_print(format_args_nl!($($arg)*));
	}};
}

/// Same as `print!`, except it appends a newline at the end.
#[cfg(userspace)]
#[allow_internal_unstable(print_internals, format_args_nl)]
#[macro_export]
macro_rules! println {
	() => (crate::print!("\n"));
	($($arg:tt)*) => {{
		crate::print::_print_stdout(format_args_nl!($($arg)*));
	}};
}
