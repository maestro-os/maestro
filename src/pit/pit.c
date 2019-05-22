#include "pit.h"

static schedule_t *schedules;

__attribute__((cold))
void pit_init()
{
	outb(PIT_COMMAND, PIT_SELECT_CHANNEL_0 | PIT_ACCESS_LOBYTE_HIBYTE
		| PIT_MODE_0);
	// TODO Set PIT frequency

	outb(PIT_COMMAND, PIT_SELECT_CHANNEL_2 | PIT_ACCESS_LOBYTE_HIBYTE
		| PIT_MODE_3);

	schedules = NULL;
}

// TODO cli?
__attribute__((hot))
void pit_set_count(const uint16_t count)
{
	outb(PIT_CHANNEL_0, count & 0xff);
	outb(PIT_CHANNEL_0, (count >> 8) & 0xff);
}

__attribute__((hot))
void pit_schedule(const unsigned ms, void (*handler)(void *), void *data)
{
	// TODO Dedicated cache for schedulers?
	schedule_t *s;
	if (!(s = kmalloc(sizeof(s)))) return;

	s->ms = ms;
	s->handler = handler;
	s->data = data;

	if(schedules)
	{
		schedule_t *tmp = schedules;
		while(tmp->next) tmp = tmp->next;

		tmp->next = s;
	}
	else
		schedules = s;
}

extern bool interrupt_handle();
extern void interrupt_done();

__attribute__((hot))
void pit_interrupt()
{
	if(!interrupt_handle()) return;

	// TODO

	interrupt_done();
}
