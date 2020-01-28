#include <kernel.h>
#include <tty/tty.h>
#include <pic/pic.h>
#include <process/process.h>
#include <debug/debug.h>

/*
 * The name associated with every CPU exception.
 */
ATTR_RODATA
static const char *errors[] = {
	"Divide-by-zero Error",
	"Debug",
	"Non-maskable Interrupt",
	"Breakpoint",
	"Overflow",
	"Bound Range Exceeded",
	"Invalid Opcode",
	"Device Not Available",
	"Double Fault",
	"Coprocessor Segment Overrun",
	"Invalid TSS",
	"Segment Not Present",
	"Stack-Segment Fault",
	"General Protection Fault",
	"Page Fault",
	"Unknown",
	"x87 Floating-Point Exception",
	"Alignement Check",
	"Machine Check",
	"SIMD Floating-Point Exception",
	"Virtualization Exception",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Security Exception",
	"Unknown"
};

/*
 * The list of signals associated with each CPU exception.
 */
ATTR_RODATA
static int error_signals[] = {
	SIGFPE,
	SIGTRAP, // TODO
	SIGINT, // TODO
	SIGTRAP,
	-1, // TODO
	-1, // TODO
	SIGILL,
	SIGFPE,
	-1,
	-1,
	-1,
	-1,
	-1,
	SIGSEGV,
	SIGSEGV,
	-1,
	SIGFPE,
	-1, // TODO
	-1,
	SIGFPE,
	-1,
	-1,
	-1,
	-1,
	-1,
	-1,
	-1,
	-1,
	-1,
	-1,
	-1,
	-1
};

// TODO Check if switching context
/*
 * Handles a CPU exception.
 */
void error_handler(const unsigned error, const uint32_t error_code)
{
	process_t *process;
	int sig;

	if(error > 0x1f)
		PANIC("Unknown", error_code);
	if(!(process = get_running_process()) || process->syscalling
		|| (sig = error_signals[error]) < 0)
		PANIC(errors[error], error_code);
	if(error == 0xd) // TODO and *eip == 0xf4
	{
		// TODO process_exit(process, eax);
		process_kill(process, sig); // TODO rm
	}
	else
	{
		if(error == 0xe && mem_space_handle_page_fault(process->mem_space,
			cr2_get(), error_code))
			return;
		process_kill(process, sig);
	}
	pic_EOI(error);
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
	printf("Kernel has been forced to halt due to internal problem,\
 sorry :/\n");
	printf("Reason: %s\n", reason);
	printf("Error code: %x\n", (unsigned) code);
	printf("CR2: %p\n\n", cr2_get());
	printf("If you believe this is a bug on the kernel side,\
 please feel free to report it.\n");
}

/* * Triggers a kernel panic with the specified reason and error code.
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
