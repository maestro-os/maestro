#include <kernel.h>
#include <tty/tty.h>
#include <pic/pic.h>
#include <memory/vmem/vmem.h>
#include <process/process.h>
#include <process/scheduler.h>
#include <debug/debug.h>

#define HLT_INSTRUCTION	0xf4

/*
 * The name associated with every CPU exception.
 */
ATTR_RODATA
static const char *errors[] = {
	[0] = "Divide-by-zero Error",
	[1] = "Debug",
	[2] = "Non-maskable Interrupt",
	[3] = "Breakpoint",
	[4] = "Overflow",
	[5] = "Bound Range Exceeded",
	[6] = "Invalid Opcode",
	[7] = "Device Not Available",
	[8] = "Double Fault",
	[9] = "Coprocessor Segment Overrun",
	[10] = "Invalid TSS",
	[11] = "Segment Not Present",
	[12] = "Stack-Segment Fault",
	[13] = "General Protection Fault",
	[14] = "Page Fault",
	[15] = "Unknown",
	[16] = "x87 Floating-Point Exception",
	[17] = "Alignement Check",
	[18] = "Machine Check",
	[19] = "SIMD Floating-Point Exception",
	[20] = "Virtualization Exception",
	[21] = "Unknown",
	[22] = "Unknown",
	[23] = "Unknown",
	[24] = "Unknown",
	[25] = "Unknown",
	[26] = "Unknown",
	[27] = "Unknown",
	[28] = "Unknown",
	[29] = "Unknown",
	[30] = "Security Exception",
	[31] = "Unknown"
};

/*
 * The list of signals associated with each CPU exception.
 */
ATTR_RODATA
static int error_signals[] = {
	[0] = SIGFPE,
	[1] = SIGTRAP, // TODO
	[2] = SIGINT, // TODO
	[3] = SIGTRAP,
	[4] = -1, // TODO
	[5] = -1, // TODO
	[6] = SIGILL,
	[7] = SIGFPE,
	[8] = -1,
	[9] = -1,
	[10] = -1,
	[11] = -1,
	[12] = -1,
	[13] = SIGSEGV,
	[14] = SIGSEGV,
	[15] = -1,
	[16] = SIGFPE,
	[17] = -1, // TODO
	[18] = -1,
	[19] = SIGFPE,
	[20] = -1,
	[21] = -1,
	[22] = -1,
	[23] = -1,
	[24] = -1,
	[25] = -1,
	[26] = -1,
	[27] = -1,
	[28] = -1,
	[29] = -1,
	[30] = -1,
	[31] = -1
};

// TODO Check if switching context
/*
 * Handles a CPU exception.
 */
void error_handler_(unsigned error, uint32_t error_code, const regs_t *regs)
{
	vmem_t page_dir;
	process_t *process;
	int sig;

	page_dir = cr3_get();
	if(kernel_vmem)
		paging_enable(kernel_vmem);
	if(error > 0x1f)
		PANIC("Unknown", error_code);
	if(!(process = get_running_process()) || process->syscalling
		|| (sig = error_signals[error]) < 0)
		PANIC(errors[error], error_code);
	if(error == 0xd && *((uint8_t *) regs->eip) == HLT_INSTRUCTION)
		process_exit(process, regs->eax);
	else if(error == 0xe && mem_space_handle_page_fault(process->mem_space,
		cr2_get(), error_code))
	{
		pic_EOI(error); // TODO Useful?
		paging_enable(page_dir);
		return;
	}
	else
		process_kill(process, sig);
	pic_EOI(error); // TODO Useful?
	paging_enable(page_dir);
	kernel_loop();
}

/*
 * Initializes the TTY and prints a panic message.
 */
ATTR_COLD
static void print_panic(const char *reason, const uint32_t code)
{
	tty_init();
	printf("--- KERNEL PANIC ---\n\n");
	printf("Kernel has been forced to halt due to internal problem, \
sorry :/\n");
	printf("Reason: %s\n", reason);
	printf("Error code: %x\n", (unsigned) code);
	printf("CR2: %p\n\n", cr2_get());
	printf("If you believe this is a bug on the kernel side, \
please feel free to report it.\n");
}

/*
 * Triggers a kernel panic with the specified reason and error code.
 * This function should be called using `PANIC(...)` only.
 */
ATTR_COLD
ATTR_NORETURN
void kernel_panic(const char *reason, const uint32_t code)
{
	CLI();
	print_panic(reason, code);
	kernel_halt();
}

/*
 * Same function as `kernel_panic` except that it takes more arguments: the file
 * and the line number where the kernel panic was triggered.
 */
ATTR_COLD
ATTR_NORETURN
void kernel_panic_(const char *reason, const uint32_t code,
	const char *file, const int line)
{
	void *ebp;

	CLI();
	print_panic(reason, code);
	printf("\n-- DEBUG --\nFile: %s; Line: %i\n", file, line);
	if(get_running_process())
		print_regs(&get_running_process()->regs_state);
	printf("\n");
	GET_EBP(ebp);
	print_callstack(ebp, 8);
	kernel_halt();
}
