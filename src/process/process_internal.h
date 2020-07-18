#ifndef PROCESS_INTERNAL_H
# define PROCESS_INTERNAL_H

# define QUEUES_COUNT	16
# define CYCLE_LENGTH	(QUEUES_COUNT * (QUEUES_COUNT + 1) / 2)

extern avl_tree_t *processes;

extern process_t *running_process;

extern tss_entry_t tss;

extern spinlock_t processes_spinlock;

/*
 * Structure representing a scheduling queue.
 */
typedef struct
{
	/* The pointer to the next process to be executed. */
	list_head_t *cursor;
	/* The list of every processes in this queue. */
	list_head_t *list;
} schedule_queue_t;

#endif
