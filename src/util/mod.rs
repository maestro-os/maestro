use crate::memory::Void;
use crate::memory;
use crate::tty;

/* Maximum size for a signed integer */
pub type Imax = i32;
/* Maximum size for an unsigned integer */
pub type Umax = u32;

/*
 * Tells if pointer `ptr` is aligned on boundary `n`.
 */
#[inline(always)]
pub fn is_aligned(ptr: *const Void, n: usize) -> bool {
	((ptr as usize) & (n - 1)) == 0
}

/*
 * Aligns down a pointer. The retuned value shall be lower than `ptr` or equal
 * if the pointer is already aligned.
 */
#[inline(always)]
pub fn down_align(ptr: *const Void, n: usize) -> *const Void {
	((ptr as usize) & !(n - 1)) as *const Void
}

/*
 * Aligns up a pointer. The returned value shall be greater than `ptr`.
 */
#[inline(always)]
pub fn up_align(ptr: *const Void, n: usize) -> *const Void {
	((down_align(ptr, n) as usize) + n) as *const Void
}

/*
 * Aligns a pointer. The returned value shall be greater than `ptr` or equal if
 * the pointer is already aligned.
 */
#[inline(always)]
pub fn align(ptr: *const Void, n: usize) -> *const Void {
	if is_aligned(ptr, n) { ptr } else { up_align(ptr, n) }
}

/*
 * Tells whether `p0` and `p1` are on the same memory page or not.
 */
#[inline(always)]
pub fn same_page(p0: *const Void, p1: *const Void) -> bool {
	down_align(p0, memory::PAGE_SIZE) == down_align(p1, memory::PAGE_SIZE)
}

/*
 * Computes ceil(n0 / n1) without using floating point numbers.
 */
#[inline(always)]
pub fn ceil_division(n0: Umax, n1: Umax) -> Umax {
	if (n0 % n1) != 0 {
		(n0 / n1) + 1
	} else {
		n0 / n1
	}
}

/*
 * Computes 2^^n on unsigned integers (where `^^` is an exponent).
 */
#[inline(always)]
pub fn pow2(n: Umax) -> Umax {
	(1 as Umax) << n
}

/*
 * Computes floor(log2(n)) without on unsigned integers.
 */
#[inline(always)]
pub fn log2(n: Umax) -> Umax {
	if n == 0 {
		return 1;
	}
	(bit_size_of::<Umax>() as Umax) - n.leading_zeros() - 1
}

/*
 * Computes the square root of an integer.
 */
#[inline(always)]
pub fn sqrt(n: Umax) -> Umax {
	pow2(log2(n) / 2)
}

/*
 * Returns the of a type in bits.
 */
#[inline(always)]
pub fn bit_size_of<T>() -> usize {
	core::mem::size_of::<T>() * 8
}

/*
 * Returns the size of a bitfield of `n` elements in bytes.
 */
#[inline(always)]
pub fn bitfield_size(n: usize) -> usize {
	ceil_division(n as Umax, bit_size_of::<u8>() as Umax) as usize
}

/*
 * Returns the offset of the given field `field` in structure `type`.
 */
macro_rules! offset_of {
	($type:expr, $field:expr) => (&((0 as *const $type).$field) as usize)
}

/*
 * Returns the structure of type `type` that contains the structure in field
 * `field` at pointer `ptr`.
 */
macro_rules! container_of {
	($ptr:expr, $type:expr, $field:expr) => ((($ptr as usize) - offset_of($type, $field)) as *const $type)
}

/*
 * TODO
 */
struct TTYWrite {}

impl core::fmt::Write for TTYWrite {
	fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error> {
		tty::current().write(s);
		Ok(())
	}
}

/*
 * Prints the specified message on the current TTY. This function is meant to be used through `print!` and `println!`
 * macros only.
 */
pub fn _print(args: core::fmt::Arguments) {
	let mut w: TTYWrite = TTYWrite {};
	core::fmt::write(&mut w, args).ok();
}

/*
 * Prints the given formatted string with the given values.
 */
#[allow_internal_unstable(print_internals)]
#[macro_export]
macro_rules! print {
	($($arg:tt)*) => {{
		crate::util::_print(format_args!($($arg)*));
	}};
}

/*
 * Same as `print!`, except it appends a newline at the end.
 */
#[allow_internal_unstable(print_internals, format_args_nl)]
#[macro_export]
macro_rules! println {
	() => (::print!("\n"));
	($($arg:tt)*) => {{
		crate::util::_print(format_args_nl!($($arg)*));
	}};
}

/*
 * Structure representing the list of registers for a context.
 */
pub struct Regs
{
	pub ebp: i32,
	pub esp: i32,
	pub eip: i32,
	pub eflags: i32,
	pub eax: i32,
	pub ebx: i32,
	pub ecx: i32,
	pub edx: i32,
	pub esi: i32,
	pub edi: i32,
}

/*
 * Extern spinlock functions.
 */
extern "C" {
	pub fn spin_lock(lock: *mut bool);
	pub fn spin_unlock(lock: *mut bool);
}

/*
 * Structure representing a spinlock.
 */
#[derive(Copy)]
#[derive(Clone)]
pub struct Spinlock {
	locked: bool,
}

impl Spinlock {
	/*
	 * Creates a new spinlock.
	 */
	pub const fn new() -> Self {
		Self {
			locked: false,
		}
	}

	/*
	 * Wrapper for `spin_lock`. Locks the spinlock.
	 */
	pub fn lock(&mut self) {
		unsafe {
			spin_lock(&mut self.locked);
		}
	}

	/*
	 * Wrapper for `spin_unlock`. Unlocks the spinlock.
	 */
	pub fn unlock(&mut self) {
		unsafe {
			spin_unlock(&mut self.locked);
		}
	}
}
