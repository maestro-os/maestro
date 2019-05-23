#include "pit.h"

static schedule_t *schedules;

__attribute__((cold))
void pit_init()
{
	outb(PIT_COMMAND, PIT_SELECT_CHANNEL_0 | PIT_ACCESS_LOBYTE_HIBYTE
		| PIT_MODE_0);
	pit_set_frequency(1000);

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
void pit_sleep(const unsigned duration)
{
	// TODO
	(void) duration;
}

__attribute__((hot))
void pit_schedule(const unsigned duration, const unsigned repeat,
	void (*handler)(void *), void *data)
{
	if(duration == 0)
	{
		if(repeat > 0) handler(data);
		return;
	}

	// TODO Dedicated cache for schedulers?
	schedule_t *s;
	if(!(s = kmalloc(sizeof(s)))) return;

	s->base_duration = duration;
	s->remain = duration;
	s->repeat = repeat;
	s->handler = handler;
	s->data = data;

	s->next = schedules;
	schedules = s;
}

extern bool interrupt_handle();
extern void interrupt_done();

__attribute__((hot))
void pit_interrupt()
{
	if(interrupt_handle() == 0) return;

	schedule_t *s = schedules, *tmp, *prev = NULL;

	while(s)
	{
		if(--(s->remain) == 0)
		{
			s->handler(s->data);

			if(s->repeat != 0 && --(s->repeat) == 0)
			{
				if(s == schedules)
				{
					tmp = s->next;
					schedules = tmp;
				}
				else if(prev)
					prev->next = s->next;

				kfree(s);
			}
			else
				s->remain = s->base_duration;
		}

		prev = s;
		s = s->next;
	}

	interrupt_done();
}
