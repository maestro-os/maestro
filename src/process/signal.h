#ifndef SIGNAL_H
# define SIGNAL_H

# include <libc/sys/types.h>

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

typedef struct signal
{
	struct signal *next;

	int si_signo;
	int si_code;
	int si_errno;

	pid_t si_pid;
	uid_t si_uid;
	void *si_addr;
	int si_status;

	long si_band;

	// TODO si_value
} signal_t;

#endif
