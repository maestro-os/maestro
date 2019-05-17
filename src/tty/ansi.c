#include "tty.h"

static uint8_t get_nbr(const char *buffer, size_t *i, const size_t count)
{
	uint8_t n = 0;

	while(*i < count && buffer[*i] >= '0' && buffer[*i] <= '9')
		n = (n * 10) + buffer[(*i)++] - '0';
	return n;
}

static void handle_csi_code(tty_t *tty, const uint8_t code)
{
	switch(code)
	{
		case 0:
		{
			tty_reset_attrs(tty);
			break;
		}

		// TODO

		case 30:
		{
			tty_set_fgcolor(tty, VGA_COLOR_BLACK);
			break;
		}

		case 31:
		{
			tty_set_fgcolor(tty, VGA_COLOR_RED);
			break;
		}

		case 32:
		{
			tty_set_fgcolor(tty, VGA_COLOR_GREEN);
			break;
		}

		case 33:
		{
			tty_set_fgcolor(tty, VGA_COLOR_BROWN);
			break;
		}

		case 34:
		{
			tty_set_fgcolor(tty, VGA_COLOR_BLUE);
			break;
		}

		case 35:
		{
			tty_set_fgcolor(tty, VGA_COLOR_MAGENTA);
			break;
		}

		case 36:
		{
			tty_set_fgcolor(tty, VGA_COLOR_CYAN);
			break;
		}

		case 37:
		{
			tty_set_fgcolor(tty, VGA_COLOR_LIGHT_GREY);
			break;
		}

		case 40:
		{
			tty_set_bgcolor(tty, VGA_COLOR_BLACK);
			break;
		}

		case 41:
		{
			tty_set_bgcolor(tty, VGA_COLOR_RED);
			break;
		}

		case 42:
		{
			tty_set_bgcolor(tty, VGA_COLOR_GREEN);
			break;
		}

		case 43:
		{
			tty_set_bgcolor(tty, VGA_COLOR_BROWN);
			break;
		}

		case 44:
		{
			tty_set_bgcolor(tty, VGA_COLOR_BLUE);
			break;
		}

		case 45:
		{
			tty_set_bgcolor(tty, VGA_COLOR_MAGENTA);
			break;
		}

		case 46:
		{
			tty_set_bgcolor(tty, VGA_COLOR_CYAN);
			break;
		}

		case 47:
		{
			tty_set_bgcolor(tty, VGA_COLOR_LIGHT_GREY);
			break;
		}

		case 90:
		{
			tty_set_fgcolor(tty, VGA_COLOR_DARK_GREY);
			break;
		}

		case 91:
		{
			tty_set_fgcolor(tty, VGA_COLOR_LIGHT_RED);
			break;
		}

		case 92:
		{
			tty_set_fgcolor(tty, VGA_COLOR_LIGHT_GREEN);
			break;
		}

		case 93:
		{
			tty_set_fgcolor(tty, VGA_COLOR_YELLOW);
			break;
		}

		case 94:
		{
			tty_set_fgcolor(tty, VGA_COLOR_LIGHT_BLUE);
			break;
		}

		case 95:
		{
			tty_set_fgcolor(tty, VGA_COLOR_LIGHT_MAGENTA);
			break;
		}

		case 96:
		{
			tty_set_fgcolor(tty, VGA_COLOR_LIGHT_CYAN);
			break;
		}

		case 97:
		{
			tty_set_fgcolor(tty, VGA_COLOR_WHITE);
			break;
		}

		case 100:
		{
			tty_set_bgcolor(tty, VGA_COLOR_DARK_GREY);
			break;
		}

		case 101:
		{
			tty_set_bgcolor(tty, VGA_COLOR_LIGHT_RED);
			break;
		}

		case 102:
		{
			tty_set_bgcolor(tty, VGA_COLOR_LIGHT_GREEN);
			break;
		}

		case 103:
		{
			tty_set_bgcolor(tty, VGA_COLOR_YELLOW);
			break;
		}

		case 104:
		{
			tty_set_bgcolor(tty, VGA_COLOR_LIGHT_BLUE);
			break;
		}

		case 105:
		{
			tty_set_bgcolor(tty, VGA_COLOR_LIGHT_MAGENTA);
			break;
		}

		case 106:
		{
			tty_set_bgcolor(tty, VGA_COLOR_LIGHT_CYAN);
			break;
		}

		case 107:
		{
			tty_set_bgcolor(tty, VGA_COLOR_WHITE);
			break;
		}
	}
}

static void handle_csi(tty_t *tty, const char *buffer,
	size_t *i, const size_t count)
{
	++(*i);

	while(*i < count && !(buffer[*i] >= 0x40 && buffer[*i] <= 0x7e))
	{
		handle_csi_code(tty, get_nbr(buffer, i, count));

		if(buffer[*i] < 0x20) return;
		while(*i < count && (buffer[*i] == ';'
			|| (buffer[*i] >= 0x20 && buffer[*i] <= 0x2f)))
			++(*i);
	}
}

void ansi_handle(tty_t *tty, const char *buffer, size_t *i, const size_t count)
{
	if(!buffer || !i) return;
	if(buffer[*i] != ANSI_ESCAPE) return;
	++(*i);

	if(*i >= count)
	{
		tty_write(buffer + *i - 1, 1, tty);
		return;
	}

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
