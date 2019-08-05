#ifndef _CTYPE_H
# define _CTYPE_H

inline int isalpha(int c)
{
	return ((c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z'));
}

inline int isdigit(int c)
{
	return (c >= '0' && c <= '9');
}

inline int isalnum(int c)
{
	return (isalpha(c) || isdigit(c));
}

int iscntrl(int c)
{
	(void)c;
	// TODO

	return 0;
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

int ispunct(int c)
{
	(void) c;
	// TODO

	return 0;
}

inline int isspace(int c)
{
	return (c >= '\t' && c <= '\r');
}

inline int isupper(int c)
{
	return (c >= 'A' && c <= 'Z');
}

int isxdigit(int c)
{
	(void) c;
	// TODO

	return 0;
}

inline int isascii(int c)
{
	return ((c & ~0x7f) == 0);
}

int isblank(int c)
{
	(void) c;
	// TODO

	return 0;
}

#endif
