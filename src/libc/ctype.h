#ifndef _CTYPE_H
# define _CTYPE_H

int isalnum(int c);
int isalpha(int c);
int iscntrl(int c);
int isdigit(int c);
int isgraph(int c);
int islower(int c);
int isprint(int c);
int ispunct(int c);

int isspace(int c)
{
	return (c >= '\t' && c <= '\r');
}

int isupper(int c);
int isxdigit(int c);

inline int isascii(int c)
{
	return ((c & ~0x7f) == 0);
}

int isblank(int c);

#endif
