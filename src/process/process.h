#ifndef PROCESS_H
# define PROCESS_H

# include "../gdt/gdt.h"
# include "../memory/memory.h"

# include <stdint.h>

# define PID_MAX	32768

typedef int16_t pid_t;

typedef struct process
{
	pid_t pid, parent;
	// TODO data

	uint32_t *page_dir;

	struct process *next;
	struct process *prev;
} process_t;

__attribute__((packed))
struct tss_entry
{
	uint32_t prev_tss;
	uint32_t esp0;
	uint32_t ss0;
	uint32_t esp1;
	uint32_t ss1;
	uint32_t esp2;
	uint32_t ss2;
	uint32_t cr3;
	uint32_t eip;
	uint32_t eflags;
	uint32_t eax;
	uint32_t ecx;
	uint32_t edx;
	uint32_t ebx;
	uint32_t esp;
	uint32_t ebp;
	uint32_t esi;
	uint32_t edi;
	uint32_t es;
	uint32_t cs;
	uint32_t ss;
	uint32_t ds;
	uint32_t fs;
	uint32_t gs;
	uint32_t ldt;
	uint16_t trap;
	uint16_t iomap_base;
};

typedef struct tss_entry tss_entry_t;

extern void *tss_gdt_entry(void);
extern void tss_flush(void);

void process_init(void);
pid_t kfork(const pid_t parent);
process_t *get_process(const pid_t pid);
void process_tick(void);
extern void switch_usermode(void);

#endif
