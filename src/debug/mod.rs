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
	($reg:literal) => {{
		let mut val: u32;
		unsafe { llvm_asm!(concat!("mov %%", $reg, ", %0") : "=a"(val)); }
		val
	}};
}

/*
 * Prints the registers into the given `regs` structure.
 */
pub fn print_regs(regs: &util::Regs) {
	print!("ebp: {:p} ", regs.ebp as *const Void);
	print!("esp: {:p} ", regs.esp as *const Void);
	print!("eip: {:p} ", regs.eip as *const Void);
	print!("eflags: {:p} ", regs.eflags as *const Void);
	print!("eax: {:p}\n", regs.eax as *const Void);
	print!("ebx: {:p} ", regs.ebx as *const Void);
	print!("ecx: {:p} ", regs.ecx as *const Void);
	print!("edx: {:p} ", regs.edx as *const Void);
	print!("esi: {:p} ", regs.esi as *const Void);
	print!("edi: {:p}\n", regs.edi as *const Void);
}

pub fn print_memory(_src: &str, _n: usize) {
	// TODO
}

/*
 * Returns the name of the function for the given instruction pointer. If the name cannot be retrived, the function
 * returns "???".
 */
fn get_function_name(_i: *const Void) -> &'static str {
	// TODO
	"TODO"
}

pub fn print_callstack(ebp: *const u32, max_depth: usize) {
	println!("--- Callstack ---");

	let mut i: usize = 0;
	let mut ebp_ = ebp;
	let mut eip = 0 as *const Void;
	while ebp_ != 0 as *const u32 && i < max_depth {
		// TODO
		/*if !memory::vmem::is_mapped(memory::kern_to_virt(memory::cr3_get()), ebp_) {
			break;
		}*/
		unsafe {
			eip = *((ebp_ as usize + 4) as *const u32) as *const _;
		}
		if eip != (0 as *const _) {
			break;
		}
		println!("{}: {:p} -> {}", i, eip, get_function_name(eip));
		unsafe {
			ebp_ = *(ebp_ as *const u32) as *const u32;
		}
		i += 1;
	}
	if i == 0 {
		println!("Empty");
	} else if ebp_ != (0 as *const _) && eip != (0 as *const _) {
		println!("...");
	}
}

// TODO
