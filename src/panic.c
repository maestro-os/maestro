#include <kernel.h>
#include <tty/tty.h>
#include <process/process.h>

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
	-1,
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

void error_handler(const unsigned error, const uint32_t error_code)
{
	process_t *process;
	int sig;

	if(error > 0x1f)
	{
		PANIC("Unknown", error_code);
		return;
	}
	if(!(process = get_running_process()) || (sig = error_signals[error]) < 0)
	{
		PANIC(errors[error], error_code);
		return;
	}
	process_kill(process, sig);
	// TODO Do not return to process if not running anymore
}

__attribute__((cold))
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

__attribute__((cold))
__attribute((noreturn))
void kernel_panic(const char *reason, const uint32_t code)
{
	print_panic(reason, code);
	kernel_halt();
}

__attribute__((cold))
__attribute__((noreturn))
void kernel_panic_(const char *reason, const uint32_t code,
	const char *file, const int line)
{
	print_panic(reason, code);
	printf("\n-- DEBUG --\nFile: %s; Line: %i", file, line);
	kernel_halt();
}
