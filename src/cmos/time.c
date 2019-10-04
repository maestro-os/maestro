#include <cmos/cmos.h>

// TODO Use network if possible

// TODO Seconds or milliseconds?
static time_t current_time = 0;

static inline int is_leap_year(const time_t year)
{
	return (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
}

static time_t get_seconds(const time_t month, const time_t year)
{
	size_t i;
	time_t n = 0;

	for(i = EPOCH_YEAR; i < year; ++i)
		n += (is_leap_year(i) ? 366 : 365);
	for(i = 0; i < month; ++i)
		n += (i % 2 == 0 ? 31 : 30);
	return n * 86400;
}

// TODO Fix
void time_init(void)
{
	time_t seconds, minutes, hours, day, month, year, century;
	uint8_t status_b;

	// TODO Enable IRQ8
	seconds = cmos_get_time(CMOS_SECONDS_REGISTER);
	minutes = cmos_get_time(CMOS_MINUTES_REGISTER);
	hours = cmos_get_time(CMOS_HOURS_REGISTER);
	day = cmos_get_time(CMOS_DAY_OF_MONTH_REGISTER);
	month = cmos_get_time(CMOS_MONTH_REGISTER);
	year = cmos_get_time(CMOS_YEAR_REGISTER);
	// TODO Check if century can be used using ACPI
	century = cmos_get_time(CMOS_CENTURY_REGISTER);
	// TODO If it cannot be used, 19 if year >= 70, else 20
	status_b = cmos_read_register(CMOS_STATUS_B);
	if(!(status_b & 0x4))
	{
		seconds = BCD_TO_BINARY(seconds);
		minutes = BCD_TO_BINARY(minutes);
		hours = BCD_TO_BINARY(hours);
		day = BCD_TO_BINARY(day) - 1;
		month = BCD_TO_BINARY(month) - 1;
		year = BCD_TO_BINARY(year);
		century = BCD_TO_BINARY(century);
	}
	if(!(status_b & 0x02) && (hours & 0x80))
		hours = ((hours & 0x7f) + 12) % 24;
	current_time = seconds + (minutes * 60) + (hours * 3600)
		+ (day * 86400) + get_seconds(month, century * 100 + year);
}

void time_update(void)
{
	// TODO Update time with PIT
	// TODO Use a variable to store approximation due to frequency of the PIT
}

time_t time_get(void)
{
	return current_time;
}
