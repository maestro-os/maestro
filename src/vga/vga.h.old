#ifndef VGA_H
# define VGA_H

# include <kernel.h>
# include <memory/memory.h>
# include <libc/string.h>

/*
 * Physical address of the VGA text buffer.
 */
# define VGA_BUFFER_PHYS	((void *) 0xb8000)
/*
 * Virtual address of the VGA text buffer.
 */
# define VGA_BUFFER_VIRT	(PROCESS_END + (uintptr_t) VGA_BUFFER_PHYS)
# define VGA_BUFFER			VGA_BUFFER_VIRT

/*
 * Width of the screen in characters under the VGA text mode.
 */
# define VGA_WIDTH			80
/*
 * Height of the screen in characters under the VGA text mode.
 */
# define VGA_HEIGHT			25
/*
 * The size in bytes of the VGA text buffer.
 */
# define VGA_BUFFER_SIZE	(VGA_WIDTH * VGA_HEIGHT * sizeof(uint16_t))

/*
 * VGA text mode colors.
 */
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

/*
 * VGA text mode default color.
 */
# define VGA_DEFAULT_COLOR     (VGA_COLOR_WHITE | (VGA_COLOR_BLACK << 4))

# define CURSOR_START	0
# define CURSOR_END		15

typedef int8_t vgacolor_t;
typedef int16_t vgapos_t;

/*
 * Returns the value for the given foreground color `fg` and background color
 * `bg`.
 */
inline vgacolor_t vga_entry_color(vgacolor_t fg, vgacolor_t bg)
{
	return fg | (bg << 4);
}

void vga_clear(void);
void vga_enable_cursor(void);
void vga_disable_cursor(void);
void vga_move_cursor(vgapos_t x, vgapos_t y);
void vga_putchar_color(char c, uint8_t color, vgapos_t x, vgapos_t y);

/*
 * Writes the given character `c` at the given position `x`/`y` on the screen
 * with the default color.
 */
inline void vga_putchar(char c, vgapos_t x, vgapos_t y)
{
	vga_putchar_color(c, VGA_DEFAULT_COLOR, x, y);
}

#endif
