#include <rtc/rtc.h>
#include <pic/pic.h>

void rtc_init(void)
{
	int enabled;
	uint8_t prev;
	int rate = 11; // TODO Change

	enabled = interrupt_is_enabled();
	CLI();
	prev = cmos_read_register(CMOS_STATUS_B);
	cmos_write_register(CMOS_STATUS_B, prev | 0x40);
	prev = cmos_read_register(CMOS_STATUS_A);
	cmos_write_register(CMOS_STATUS_A, (prev & 0xf0) | (rate - 1));
	if(enabled)
		STI();
}

void rtc_release(void)
{
	cmos_read_register(CMOS_STATUS_C);
	pic_EOI(0x8);
}
