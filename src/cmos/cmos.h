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
# define CMOS_CENTURY_REGISTER		0x32
// TODO Detect century register with ACPI

# define CMOS_STATUS_A				0xa
# define CMOS_STATUS_B				0xb

uint8_t cmos_detect_floppy(void);
const char *cmos_get_floppy_string(uint8_t type);
uint8_t cmos_get_time(uint8_t reg);

#endif
