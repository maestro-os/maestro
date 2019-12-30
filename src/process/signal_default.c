#include <process/signal.h>
#include <process/process.h>

// TODO Remove
#define TODO	termination
#include <libc/stdio.h>

// TODO Handle multicore

static void termination(process_t *proc, const int sig)
{
	// TODO Kill children?
	(void) sig;
	proc->status = 0; // TODO
	process_set_state(proc, TERMINATED);
}

static void stop(process_t *proc, const int sig)
{
	// TODO Set status
	(void) sig;
	process_set_state(proc, STOPPED);
}

static void cont(process_t *proc, const int sig)
{
	// TODO Set status?
	(void)sig;
	if(proc->state == STOPPED)
		process_set_state(proc, WAITING);
}

static inline void sigkill_dfl(process_t *proc, const int sig)
{
	// TODO Terminate process. Not children. Make children orphan. Set exit status
	(void) proc;
	(void) sig;
}

static inline void sigstop_dfl(process_t *proc, const int sig)
{
	// TODO Set status
	(void) sig;
	proc->state = STOPPED;
	// TODO Wait until process switch or perform it now?
}

static void (*const funcs[])(process_t *, int) = {
	[SIGHUP] = termination,
	[SIGINT] = termination,
	[SIGQUIT] = TODO,
	[SIGILL] = TODO,
	[SIGTRAP] = TODO,
	[SIGABRT] = TODO,
	[SIGBUS] = TODO,
	[SIGFPE] = TODO,
	[SIGKILL] = sigkill_dfl,
	[SIGUSR1] = termination,
	[SIGSEGV] = TODO,
	[SIGUSR2] = termination,
	[SIGPIPE] = termination,
	[SIGALRM] = termination,
	[SIGTERM] = termination,
	[SIGCHLD] = NULL,
	[SIGCONT] = cont,
	[SIGSTOP] = stop,
	[SIGTSTP] = stop,
	[SIGTTIN] = stop,
	[SIGTTOU] = stop,
	[SIGURG] = NULL,
	[SIGXCPU] = TODO,
	[SIGXFSZ] = TODO,
	[SIGVTALRM] = termination,
	[SIGPROF] = termination,
	[SIGPOLL] = termination,
	[SIGSYS] = TODO
};

ATTR_HOT
void signal_default(process_t *proc, const int sig)
{
	void (*func)(process_t *, int);

	if(!proc || sig >= SIG_MAX || !(func = funcs[sig]))
		return;
	func(proc, sig);
}
