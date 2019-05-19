#ifndef PIT_H
# define PIT_H

# define PIT_CHANNEL_0	0x40
# define PIT_CHANNEL_1	0x41
# define PIT_CHANNEL_2	0x42
# define PIT_COMMAND	0x43

# define PIT_SELECT_CHANNEL_0	0x0
# define PIT_SELECT_CHANNEL_1	0x40
# define PIT_SELECT_CHANNEL_2	0x80
# define PIT_READ_BACK_COMMAND	0xc0

# define PIT_ACCESS_LATCH_COUNT_VALUE	0x0
# define PIT_ACCESS_LOBYTE				0x10
# define PIT_ACCESS_HIBYTE				0x20
# define PIT_ACCESS_LOBYTE_HIBYTE		0x30

# define PIT_MODE_0		0x0
# define PIT_MODE_1		0x1
# define PIT_MODE_2		0x2
# define PIT_MODE_3		0x3
# define PIT_MODE_4		0x4
# define PIT_MODE_5		0x5

// TODO

#endif
