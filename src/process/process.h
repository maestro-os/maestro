#ifndef PROCESS_H
# define PROCESS_H

# include <stdint.h>

# define PID_MAX			32768

typedef int16_t pid_t;

extern void switch_usermode(void);

typedef struct process
{
	pid_t pid, parent;
	// TODO data

	uint32_t *page_dir;

	struct process *next;
	struct process *prev;
} process_t;

void process_init(void);
pid_t kfork(const pid_t parent);
process_t *get_process(const pid_t pid);

#endif
