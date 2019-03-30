#include "tty.h"

static uint8_t get_nbr(const char **buffer)
{
	uint8_t n = 0;

	while(**buffer >= '0' && **buffer <= '9')
		n = (n * 10) + *((*buffer)++) - '0';
	return n;
}

static void handle_csi(tty_t *tty, const char *buffer,
	size_t *i, const size_t count)
{
	++(*i);

	// TODO
	(void) tty;
	(void) buffer;
	(void) count;
	(void) get_nbr;
}

void ansi_handle(tty_t *tty, const char *buffer, size_t *i, const size_t count)
{
	if(!buffer || !i) return;
	if(buffer[*i] != ANSI_ESCAPE) return;
	++(*i);

	if(*i >= count)
	{
		tty_write(buffer + *i - 1, 1); // TODO putchar on this specific `tty`
		return;
	}

	(void)tty;
	// TODO

	switch(buffer[*i])
	{
		case 'N':
		{
			// TODO SS2
			break;
		}

		case 'O':
		{
			// TODO SS3
			break;
		}

		case 'P':
		{
			// TODO DCS
			break;
		}

		case '[':
		{
			handle_csi(tty, buffer, i, count);
			break;
		}

		case '\\':
		{
			// TODO ST
			break;
		}

		case ']':
		{
			// TODO OSC
			break;
		}

		case 'X':
		{
			// TODO SOS
			break;
		}

		case '^':
		{
			// TODO PM
			break;
		}

		case '_':
		{
			// TODO APC
			break;
		}

		case 'c':
		{
			// TODO RIS
			break;
		}
	}
}
