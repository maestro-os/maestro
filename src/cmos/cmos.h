#ifndef CMOS_H
# define CMOS_H

# include <kernel.h>
# include <stdint.h>

# define CMOS_COMMAND			0x70
# define CMOS_DATA				0x71

# define CMOS_FLOPPY_REGISTER	0x10

# define FLOPPY_MASTER(state)	(((state) >> 4) & 0xf)
# define FLOPPY_SLAVE(state)	((state) & 0xf)

# define CMOS_SECONDS_REGISTER		0x00
# define CMOS_MINUTES_REGISTER		0x02
# define CMOS_HOURS_REGISTER		0x04
# define CMOS_WEEKDAY_REGISTER		0x06
# define CMOS_DAY_OF_MONTH_REGISTER	0x07
# define CMOS_MONTH_REGISTER		0x08
# define CMOS_YEAR_REGISTER			0x09
// TODO Detect century register with ACPI

const char *floppy_types[] = {
	"no drive",
	"360 KB 5.25 drive",
	"1.2 MB 5.25 drive",
	"720 KB 3.5 drive",
	"1.44 MB 3.5 drive",
	"2.88 MB 3.5 drive"
};

uint8_t cmos_detect_floppy(void);
uint8_t cmos_get_time(const uint8_t reg);

#endif
