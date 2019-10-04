#ifndef SIGNAL_H
# define SIGNAL_H

# include <kernel.h>

# define SIGHUP		1
# define SIGINT		2
# define SIGQUIT	3
# define SIGILL		4
# define SIGTRAP	5
# define SIGABRT	6
# define SIGBUS		7
# define SIGFPE		8
# define SIGKILL	9
# define SIGUSR1	10
# define SIGSEGV	11
# define SIGUSR2	12
# define SIGPIPE	13
# define SIGALRM	14
# define SIGTERM	15
# define SIGCHLD	17
# define SIGCONT	18
# define SIGSTOP	19
# define SIGTSTP	20
# define SIGTTIN	21
# define SIGTTOU	22
# define SIGURG		23
# define SIGXCPU	24
# define SIGXFSZ	25
# define SIGVTALRM	26
# define SIGPROF	27
# define SIGPOLL	29
# define SIGSYS		31

# define SIG_MAX	32

# define SIG_DFL	((sighandler_t) 0)
# define SIG_IGN	((sighandler_t) 1)

typedef int pid_t;
typedef void (*sighandler_t)(int);
typedef int sigset_t;

// TODO Move to user handling
typedef int uid_t;
typedef int gid_t;

union sigval
{
	int sival_int;
	void *sival_ptr;
};

typedef struct siginfo
{
	int si_signo;
	int si_code;
	int si_errno;

	pid_t si_pid;
	uid_t si_uid;
	void *si_addr;
	int si_status;

	long si_band;

	union sigval si_value;
} siginfo_t;

typedef struct sigaction
{
	void (*sa_handler)(int);
	sigset_t sa_mask;
	int sa_flags;
	void (*sa_sigaction)(int, siginfo_t *, void *);
} sigaction_t;

typedef struct signal
{
	struct signal *next;
	siginfo_t info;
} signal_t;

typedef struct process process_t;

void signal_default(process_t *proc, const int sig);

#endif
