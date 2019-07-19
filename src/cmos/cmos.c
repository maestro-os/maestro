#include <cmos/cmos.h>

__attribute__((hot))
static void cmos_select(const uint8_t reg)
{
	outb((1 << 7) | CMOS_COMMAND, reg);
	// TODO io_wait?
}

__attribute__((hot))
uint8_t cmos_detect_floppy(void)
{
	cmos_select(CMOS_FLOPPY_REGISTER);
	return inb(CMOS_DATA);
}

__attribute__((hot))
uint8_t cmos_get_time(const uint8_t reg)
{
	cmos_select(reg);
	return inb(CMOS_DATA);
}
