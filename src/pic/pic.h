#ifndef PIC_H
# define PIC_H

# include <stdint.h>

# define PIC_MASTER			0x20
# define PIC_SLAVE			0xa0
# define PIC_MASTER_COMMAND	PIC_MASTER
# define PIC_MASTER_DATA	(PIC_MASTER + 1)
# define PIC_SLAVE_COMMAND	PIC_SLAVE
# define PIC_SLAVE_DATA		(PIC_SLAVE + 1)

# define PIC_COMMAND_INIT	0x11
# define PIC_COMMAND_EOI	0x20

void pic_init();
void pic_EOI(const uint8_t irq);

#endif
