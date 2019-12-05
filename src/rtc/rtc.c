#include <rtc/rtc.h>
#include <pic/pic.h>

void rtc_init(void)
{
	int enabled;
	uint8_t prev;

	enabled = interrupt_is_enabled();
	CLI();
	prev = cmos_read_register(CMOS_STATUS_B);
	cmos_write_register(CMOS_STATUS_B, prev | 0x40);
	if(enabled)
		STI();
}

void rtc_release(void)
{
	cmos_read_register(CMOS_STATUS_C);
	pic_EOI(0x8);
}
