#ifndef PROCESS_INTERNAL_H
# define PROCESS_INTERNAL_H

/*
 * The size of a cycle in nanoseconds.
 */
# define CYCLE_LENGTH	6000000

extern avl_tree_t *processes;

extern process_t *running_process;

extern tss_entry_t tss;

extern spinlock_t processes_spinlock;

#endif
