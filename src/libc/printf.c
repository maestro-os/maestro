#include <libc/ctype.h>
#include <libc/stdio.h>
#include <libc/stdlib.h>
#include <tty/tty.h>

typedef struct
{
	size_t size;

	uint8_t flags;

	char parameter_width;
	size_t width;

	char parameter_precision;
	size_t precision;

	uint8_t length;
	char type;
} specifier_t;

typedef struct
{
	char c;
	int (*f)(const specifier_t *, va_list *);
} handler_t;

static void skip_nbr(const char **str)
{
	while(**str && isdigit(**str))
		++(*str);
}

static const char *next_specifier(const char *format, specifier_t *specifier)
{
	const char *begin;

	bzero(specifier, sizeof(specifier_t));
	while(*format && *format != '%')
		++format;
	if(!*format)
		return NULL;
	begin = format;
	++format;
	if(memchr("-+ 0#", *format, 1))
		specifier->flags = *(format++);
	if(*format == '*')
	{
		specifier->parameter_width = 1;
		++format;
	}
	else
	{
		specifier->width = atoi(format);
		skip_nbr(&format);
	}
	if(*format == '.')
	{
		++format;
		if(*format == '*')
		{
			specifier->parameter_width = 1;
			++format;
		}
		else
		{
			specifier->width = atoi(format);
			skip_nbr(&format);
		}
	}
	// TODO
	specifier->type = *format;
	++format;
	specifier->size = format - begin;
	return begin;
}

static inline int putchar(const char c)
{
	tty_write(&c, 1, current_tty);
	return 1;
}

static inline char get_number_char(int n)
{
	if(n < 0)
		n = -n;
	return (n < 10 ? '0' : 'a' - 10) + n;
}

static inline int putint(int n, const unsigned base)
{
	if(n >= (int) base || n <= -((int) base))
		return putint(n / base, base) + putchar(get_number_char(n % base));
	if(n < 0)
	{
		putchar('-');
		n = -n;
	}
	return putchar(get_number_char(n % base));
}

static inline int putuint(const unsigned int n, const unsigned base)
{
	if(n >= base)
		return putuint(n / base, base) + putchar(get_number_char(n % base));
	else
		return putchar(get_number_char(n % base));
}

static inline int putptr(const uintptr_t n)
{
	if(n >= 16)
		return putptr(n / 16) + putchar(get_number_char(n % 16));
	else
		return putchar(get_number_char(n % 16));
}

static inline int putfloat(const unsigned int n)
{
	(void) n;
	// TODO

	return 0;
}

static inline int putstr(const char *str)
{
	size_t len;

	len = strlen(str);
	tty_write(str, len, current_tty);
	return len;
}

static int char_handler(const specifier_t *specifier, va_list *args)
{
	// TODO Alignements, etc...
	(void) specifier;
	return putchar(va_arg(*args, int));
}

static int signed_decimal_handler(const specifier_t *specifier, va_list *args)
{
	// TODO Alignements, etc...
	(void) specifier;
	return putint(va_arg(*args, int), 10);
}

static int unsigned_decimal_handler(const specifier_t *specifier, va_list *args)
{
	// TODO Alignements, etc...
	(void) specifier;
	return putuint(va_arg(*args, int), 10);
}

static int float_handler(const specifier_t *specifier, va_list *args)
{
	// TODO Alignements, etc...
	(void) specifier;
	return putfloat(va_arg(*args, double));
}

static int string_handler(const specifier_t *specifier, va_list *args)
{
	// TODO Alignements, etc...
	(void) specifier;
	return putstr(va_arg(*args, const char *));
}

static int pointer_handler(const specifier_t *specifier, va_list *args)
{
	// TODO Alignements, etc...
	(void) specifier;
	return putstr("0x") + putptr((uintptr_t) va_arg(*args, void *));
}

static int hexadecimal_handler(const specifier_t *specifier, va_list *args)
{
	// TODO Alignements, etc...
	(void) specifier;
	return putuint((unsigned) va_arg(*args, void *), 16);
}

static int handle_specifier(const specifier_t *specifier, va_list *args)
{
	static handler_t handlers[] = {
		{'d', signed_decimal_handler},
		{'i', signed_decimal_handler},
		{'u', unsigned_decimal_handler},
		{'f', float_handler},
		{'c', char_handler},
		{'s', string_handler},
		{'p', pointer_handler},
		{'x', hexadecimal_handler}
		// TODO
	};
	size_t i = 0;
	const handler_t *h;

	if(specifier->type == '%')
	{
		putchar('%');
		return 1;
	}
	while(i < sizeof(handlers) / sizeof(handler_t))
	{
		h = handlers + i++;
		if(h->c == specifier->type)
			return h->f(specifier, args);
	}
	// TODO Do something?
	return 0;
}

int printf(const char *format, ...)
{
	int total = 0;
	va_list args;
	const char *s;
	specifier_t specifier;
	size_t len;

	va_start(args, format);
	while(*format)
	{
		s = next_specifier(format, &specifier);
		len	= (s ? (size_t) (s - format) : strlen(format));
		tty_write(format, len, current_tty);
		format += len;
		total += len;
		if(s)
			total += handle_specifier(&specifier, &args);
		format += specifier.size;
	}
	va_end(args);
	return total;
}
