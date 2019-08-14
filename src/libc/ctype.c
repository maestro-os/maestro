#include <libc/ctype.h>

int isalpha(const int c)
{
	return ((c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z'));
}

int isdigit(const int c)
{
	return (c >= '0' && c <= '9');
}

int isalnum(const int c)
{
	return (isalpha(c) || isdigit(c));
}

int iscntrl(const int c)
{
	return (isascii(c) && (c <= 0x1f || c == 0x7f));
}

int isgraph(const int c)
{
	return (c > ' ' && c < 0x7f);
}

int islower(const int c)
{
	return (c >= 'a' && c <= 'z');
}

int isprint(const int c)
{
	return (c >= ' ' && c < 0x7f);
}

int ispunct(const int c)
{
	return (isprint(c) && c != ' ' && !isalnum(c));
}

int isspace(const int c)
{
	return (c >= '\t' && c <= '\r');
}

int isupper(const int c)
{
	return (c >= 'A' && c <= 'Z');
}

int isxdigit(const int c)
{
	return (isdigit(c) || (c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F'));
}

int isascii(const int c)
{
	return ((c & ~0x7f) == 0);
}

int isblank(const int c)
{
	return (c == ' ' || c == '\t');
}
