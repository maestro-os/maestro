#ifndef VGA_H
# define VGA_H

# include "../libc/string.h"

# define TEXT_BUFFER	0xb8000
# define TEXT_WIDTH		0xb8000
# define TEXT_HEIGHT	0xb8000

# define TEXT_DEFAULT_COLOR     VGA_COLOR_WHITE | (VGA_COLOR_BLACK << 4)

typedef enum text_color
{
	VGA_COLOR_BLACK = 0,
	VGA_COLOR_BLUE = 1,
	VGA_COLOR_GREEN = 2,
	VGA_COLOR_CYAN = 3,
	VGA_COLOR_RED = 4,
	VGA_COLOR_MAGENTA = 5,
	VGA_COLOR_BROWN = 6,
	VGA_COLOR_LIGHT_GREY = 7,
	VGA_COLOR_DARK_GREY = 8,
	VGA_COLOR_LIGHT_BLUE = 9,
	VGA_COLOR_LIGHT_GREEN = 10,
	VGA_COLOR_LIGHT_CYAN = 11,
	VGA_COLOR_LIGHT_RED = 12,
	VGA_COLOR_LIGHT_MAGENTA = 13,
	VGA_COLOR_LIGHT_BROWN = 14,
	VGA_COLOR_WHITE = 15
} vga_color_t;

inline uint8_t text_entry_color(const vga_color_t fg, const vga_color_t bg)
{
	return fg | (bg << 4);
}

void text_clear();
void text_putchar_color(const char c, const uint8_t color,
	const size_t x, const size_t y);

inline void text_putchar(const char c, const size_t x, const size_t y)
{
	text_putchar_color(c, TEXT_DEFAULT_COLOR, x, y);
}

#endif
