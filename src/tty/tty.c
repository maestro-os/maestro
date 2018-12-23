#include "../kernel.h"
#include "tty.h"
#include "../framebuffer/framebuffer.h"

void tty_init()
{
	// TODO Switch from graphical to text mode if needed

	text_clear();
}

void tty_write(const char* buffer, const size_t size)
{
	static size_t cursor_x = 0;
	static size_t cursor_y = 0;

	for(size_t i = 0; i < size; ++i) {
		switch(buffer[i]) {
			case '\n': {
				cursor_x = 0;
				++cursor_y;
				break;
			}

			case '\r': {
				cursor_x = 0;
				break;
			}

			default: {
				text_putchar(buffer[i], cursor_x, cursor_y);

				if(cursor_x + 1 < boot_info.framebuffer_width) {
					++cursor_x;
				} else {
					cursor_x = 0;
					++cursor_y;
				}

				break;
			}
		}
	}
}
