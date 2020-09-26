/*
 * TODO doc
 */

use crate::memory::Void;
use crate::util;

/*
 * Returns the value into the specified register.
 */
#[macro_export]
macro_rules! register_get {
	($reg:expr) => {{
		let mut val: u32;
		llvm_asm!(concat!("mov %", $reg, ", %eax") : "={eax}"(val));

		val
	}};
}

/*
 * Prints the registers into the given `regs` structure.
 */
pub fn print_regs(regs: &util::Regs) {
	::print!("ebp: {:p} ", regs.ebp as *const Void);
	::print!("esp: {:p} ", regs.esp as *const Void);
	::print!("eip: {:p} ", regs.eip as *const Void);
	::print!("eflags: {:p} ", regs.eflags as *const Void);
	::print!("eax: {:p}\n", regs.eax as *const Void);
	::print!("ebx: {:p} ", regs.ebx as *const Void);
	::print!("ecx: {:p} ", regs.ecx as *const Void);
	::print!("edx: {:p} ", regs.edx as *const Void);
	::print!("esi: {:p} ", regs.esi as *const Void);
	::print!("edi: {:p}\n", regs.edi as *const Void);
}

/*
 * Prints, in hexadecimal, the content of the memory at the given location `ptr`, with the given
 * size `n` in bytes.
 */
pub unsafe fn print_memory(ptr: *const Void, n: usize) {
	let mut i = 0;
	while i < n {
		::print!("{:p}  ", ptr);

		let mut j = 0;
		while j < 16 && i + j < n {
			::println!("{:x?} ", *(((ptr as usize) + (i + j)) as *const u8));
			j += 1;
		}

		::print!(" |");

		j = 0;
		while j < 16 && i + j < n {
			let v = *(((ptr as usize) + (i + j)) as *const u8);
			let c = if v < 32 {
				'.'
			} else {
				v as char
			};
			::println!("{}", c);
			j += 1;
		}

		::println!("|");

		i += j;
	}
}

/*
 * Returns the name of the function for the given instruction pointer. If the name cannot be
 * retrived, the function returns "???".
 */
fn get_function_name(_i: *const Void) -> &'static str {
	// TODO
	"TODO"
}

/*
 * Prints the callstack in the current context, including symbol's name and address. `ebp` is value
 * of the `%ebp` register that is used as a starting point for printing. `max_depth` is the maximum
 * depth of the stack to print. If the stack is larger than the maximum depth, the function shall
 * print `...` at the end. If the callstack is empty, the function just prints `Empty`.
 */
pub fn print_callstack(ebp: *const u32, max_depth: usize) {
	::println!("--- Callstack ---");

	let mut i: usize = 0;
	let mut ebp_ = ebp;
	while ebp_ != 0 as *const u32 && i < max_depth {
		// TODO
		/*if !memory::vmem::is_mapped(memory::kern_to_virt(memory::cr3_get()), ebp_) {
			break;
		}*/
		let eip = unsafe {
			*((ebp_ as usize + core::mem::size_of::<usize>()) as *const u32) as *const _
		};
		if eip == (0 as *const _) {
			break;
		}
		::println!("{}: {:p} -> {}", i, eip, get_function_name(eip));
		unsafe {
			ebp_ = *(ebp_ as *const u32) as *const u32;
		}
		i += 1;
	}
	if i == 0 {
		::println!("Empty");
	} else if ebp_ != (0 as *const _) {
		::println!("...");
	}
}
