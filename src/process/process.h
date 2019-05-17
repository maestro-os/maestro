#ifndef PROCESS_H
# define PROCESS_H

# include <stdint.h>

# define PID_MAX	32768

# define LIST_PROCESS(list)	((process_t *) list->content)
# define LIST_PID(list)		(LIST_PROCESS(list)->pid)

typedef int16_t pid_t;

extern void switch_usermode();

typedef struct
{
	pid_t pid, parent;
	// TODO data

	uint32_t *page_dir;
} process_t;

void process_init();
pid_t kfork(const pid_t parent);
process_t *get_process(const pid_t pid);

#endif
