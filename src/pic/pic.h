#ifndef PIC_H
# define PIC_H

#include "../kernel.h"

# define PIC_MASTER_COMMAND	0x20
# define PIC_MASTER_DATA	0x21
# define PIC_SLAVE_COMMAND	0xa0
# define PIC_SLAVE_DATA		0xa1

# define ICW1_ICW4			0x01
# define ICW1_SINGLE		0x02
# define ICW1_INTERVAL4		0x04
# define ICW1_LEVEL			0x08
# define ICW1_INIT			0x10

# define ICW3_SLAVE_PIC		0x04
# define ICW3_CASCADE		0x02

# define ICW4_8086			0x01
# define ICW4_AUTO			0x02
# define ICW4_BUF_SLAVE		0x08
# define ICW4_BUF_MASTER	0x0C
# define ICW4_SFNM			0x10

# define PIC_COMMAND_EOI	0x20

void pic_init(const uint8_t offset1, const uint8_t offset2);
void pic_EOI(const uint8_t irq);

#endif
