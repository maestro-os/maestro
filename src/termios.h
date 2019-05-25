#ifndef TERMIOS_H
# define TERMIOS_H

// TODO
# define NCCS	0

typedef unsigned cc_t;
typedef unsigned speed_t;
typedef unsigned tcflag_t;

struct termios
{
	tcflag_t c_iflag;
	tcflag_t c_oflag;
	tcflag_t c_cflag;
	tcflag_t c_lflag;
	cc_t c_cc[NCCS];
};

// TODO

#endif
