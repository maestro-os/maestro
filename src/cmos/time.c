#include <cmos/cmos.h>

// TODO Use network if possible

// TODO Seconds or milliseconds?
static time_t current_time = 0;

static inline int is_leap_year(const time_t year)
{
	return (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
}

static time_t get_seconds(const time_t month, time_t year)
{
	size_t i;
	time_t n = 0;

	for(i = 1; i < month; ++i)
	{
		if(i == 2)
			n += (is_leap_year(year) ? 29 : 28);
		else
			n += ((i & 1) == 0 ? 31 : 30);
	}
	n *= 86400;
	year -= 1900;
	n += (year - 70) * 31536000 + ((year - 69) / 4) * 86400
		- ((year - 1) / 100) * 86400 + ((year + 299) / 400) * 86400;
	return n;
}

// TODO Fix
void time_init(void)
{
	time_t seconds, minutes, hours, day, month, year, century;
	uint8_t status_b;

	cmos_wait_ready();
	seconds = cmos_read_register(CMOS_SECONDS_REGISTER);
	minutes = cmos_read_register(CMOS_MINUTES_REGISTER);
	hours = cmos_read_register(CMOS_HOURS_REGISTER);
	day = cmos_read_register(CMOS_DAY_OF_MONTH_REGISTER);
	month = cmos_read_register(CMOS_MONTH_REGISTER);
	year = cmos_read_register(CMOS_YEAR_REGISTER);
	// TODO Check if century can be used using ACPI
	// TODO If it cannot be used, 19 if year >= 70, else 20
	century = cmos_read_register(CMOS_CENTURY_REGISTER);
	status_b = cmos_read_register(CMOS_STATUS_B);
	if(!(status_b & 0x4))
	{
		seconds = (seconds & 0x0f) + ((seconds / 16) * 10);
		minutes = (minutes & 0x0f) + ((minutes / 16) * 10);
		hours = (((hours & 0x0f) + (((hours & 0x70) / 16) * 10))
			| (hours & 0x80)) + 1;
		day = (day & 0x0f) + ((day / 16) * 10);
		month = (month & 0x0f) + ((month / 16) * 10);
		year = (year & 0x0f) + ((year / 16) * 10);
		century = (century & 0x0f) + ((century / 16) * 10);
	}
	if(!(status_b & 0x2) && (hours & 0x80))
		hours = ((hours & 0x7f) + 12) % 24;
	printf("%lu:%lu:%lu %lu-%lu-%lu\n", hours, minutes, seconds, month, day, year); // TODO rm
	year += century * 100;
	current_time = seconds + (minutes * 60) + (hours * 3600)
		+ ((day + 1) * 86400) + get_seconds(month, year); // TODO Clean
	pit_set_frequency(1000); // TODO Change?
}

void time_update(void)
{
	static size_t count = 0;

	if(count >= 1000)
	{
		count = 0;
		++current_time;
	}
	else
		++count;
	// TODO Use a variable to store approximation due to frequency of the PIT
}

time_t time_get(void)
{
	return current_time;
}
