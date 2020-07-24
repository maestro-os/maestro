#ifndef PROCESS_INTERNAL_H
# define PROCESS_INTERNAL_H

/*
 * The buddy order of interrupt stacks.
 */
# define INTERRUPT_STACK_ORDER	3

/*
 * The size of a cycle in nanoseconds.
 */
# define CYCLE_LENGTH	6000000

extern avl_tree_t *processes;

extern process_t *running_process;

extern tss_entry_t tss;

extern void **interrupt_stacks;

extern spinlock_t processes_spinlock;

#endif
