#ifndef RTC_H
# define RTC_H

# include <kernel.h>
# include <cmos/cmos.h>
# include <idt/idt.h>

void rtc_init(void);
// TODO rtc_set_frequency

void rtc_release(void);

#endif
