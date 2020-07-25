#ifndef SCHEDULER_H
# define SCHEDULER_H

# include <util/util.h>

typedef struct regs regs_t;
typedef struct process process_t;

process_t *get_running_process(void);

void scheduler_add(process_t *p);
void scheduler_remove(process_t *p);

void scheduler_tick(const regs_t *registers);

ATTR_NORETURN
extern void context_switch(const regs_t *regs,
	uint16_t data_selector, uint16_t code_selector);
ATTR_NORETURN
extern void kernel_switch(const regs_t *regs);

#endif
