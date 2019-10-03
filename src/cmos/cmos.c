#include <cmos/cmos.h>

static const char *const floppy_types[] = {
	"no drive",
	"360 KB 5.25 drive",
	"1.2 MB 5.25 drive",
	"720 KB 3.5 drive",
	"1.44 MB 3.5 drive",
	"2.88 MB 3.5 drive"
};

__attribute__((hot))
static void cmos_select(const uint8_t reg)
{
	outb(CMOS_COMMAND, reg | (1 << 7));
	// TODO io_wait?
}

__attribute__((hot))
uint8_t cmos_detect_floppy(void)
{
	cmos_select(CMOS_FLOPPY_REGISTER);
	return inb(CMOS_DATA);
}

__attribute__((hot))
const char *cmos_get_floppy_string(const uint8_t type)
{
	return floppy_types[type];
}

static int cmos_check_update(void)
{
	cmos_select(CMOS_STATUS_A);
	return (inb(CMOS_DATA) & (1 << 7));
}

static void cmos_wait_ready(void)
{
	// TODO Wait for IRQ8
	while(cmos_check_update())
		;
}

__attribute__((hot))
uint8_t cmos_get_time(const uint8_t reg)
{
	cmos_select(reg);
	cmos_wait_ready();
	return inb(CMOS_DATA);
}
