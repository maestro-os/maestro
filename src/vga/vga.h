#ifndef VGA_H
# define VGA_H

# include "../kernel.h"
# include "../libc/string.h"

# define VGA_WIDTH			80
# define VGA_HEIGHT			25
# define VGA_BUFFER			((void *) 0xb8000)
# define VGA_BUFFER_SIZE	(VGA_WIDTH * VGA_HEIGHT * sizeof(uint16_t))

# define VGA_COLOR_BLACK			0x0
# define VGA_COLOR_BLUE				0x1
# define VGA_COLOR_GREEN			0x2
# define VGA_COLOR_CYAN				0x3
# define VGA_COLOR_RED				0x4
# define VGA_COLOR_MAGENTA			0x5
# define VGA_COLOR_BROWN			0x6
# define VGA_COLOR_LIGHT_GREY		0x7
# define VGA_COLOR_DARK_GREY		0x8
# define VGA_COLOR_LIGHT_BLUE		0x9
# define VGA_COLOR_LIGHT_GREEN		0xa
# define VGA_COLOR_LIGHT_CYAN		0xb
# define VGA_COLOR_LIGHT_RED		0xc
# define VGA_COLOR_LIGHT_MAGENTA	0xd
# define VGA_COLOR_YELLOW			0xe
# define VGA_COLOR_WHITE			0xf

# define VGA_DEFAULT_COLOR     (VGA_COLOR_WHITE | (VGA_COLOR_BLACK << 4))

# define CURSOR_START	0
# define CURSOR_END		15

typedef int8_t vgacolor_t;
typedef uint16_t vgapos_t;

inline vgacolor_t vga_entry_color(const vgacolor_t fg, const vgacolor_t bg)
{
	return fg | (bg << 4);
}

void vga_clear(void);
void vga_enable_cursor(void);
void vga_disable_cursor(void);
void vga_move_cursor(const vgapos_t x, const vgapos_t y);
void vga_putchar_color(const char c, const uint8_t color,
	const vgapos_t x, const vgapos_t y);

inline void vga_putchar(const char c, const vgapos_t x, const vgapos_t y)
{
	vga_putchar_color(c, VGA_DEFAULT_COLOR, x, y);
}

#endif
