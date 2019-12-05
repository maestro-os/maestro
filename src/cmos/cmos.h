#ifndef CMOS_H
# define CMOS_H

# include <kernel.h>
# include <idt/idt.h>
# include <pit/pit.h>

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

# define CMOS_STATUS_A				0xa
# define CMOS_STATUS_B				0xb
# define CMOS_STATUS_C				0xc

# define EPOCH_YEAR			1970

typedef uint32_t time_t;

uint8_t cmos_detect_floppy(void);
const char *cmos_get_floppy_string(uint8_t type);
void cmos_wait_ready(void);
uint8_t cmos_read_register(uint8_t reg);
void cmos_write_register(uint8_t reg, uint8_t value);

void time_init(void);
void time_update(void);
time_t time_get(void);

#endif
