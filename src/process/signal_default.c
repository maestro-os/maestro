#include <process/signal.h>
#include <process/process.h>

// TODO Remove
#define TODO	termination

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
	// TODO
	(void) proc;
	(void) sig;
}

static void cont(process_t *proc, const int sig)
{
	// TODO
	(void) proc;
	(void) sig;
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

static void (*funcs[])(process_t *, int) = {
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

__attribute__((hot))
void signal_default(process_t *proc, const int sig)
{
	void (*func)(process_t *, int);

	if(!proc || sig >= SIG_MAX || !(func = funcs[sig]))
		return;
	func(proc, sig);
}
