#ifndef _CTYPE_H
# define _CTYPE_H

inline int isalnum(int c)
{
	return (isalpha(c) || isdigit(c));
}

inline int isalpha(int c)
{
	return ((c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z'));
}

int iscntrl(int c);

inline int isdigit(int c)
{
	return (c >= '0' && c <= '9');
}

inline int isgraph(int c)
{
	return (c > ' ' && c < 0x7f);
}

inline int islower(int c)
{
	return (c >= 'a' && c <= 'z');
}

inline int isprint(int c)
{
	return (c >= ' ' && c < 0x7f);
}

int ispunct(int c);

inline int isspace(int c)
{
	return (c >= '\t' && c <= '\r');
}

inline int isupper(int c)
{
	return (c >= 'A' && c <= 'Z');
}

int isxdigit(int c);

inline int isascii(int c)
{
	return ((c & ~0x7f) == 0);
}

int isblank(int c);

#endif
