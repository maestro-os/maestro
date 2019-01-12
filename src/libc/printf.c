#include "stdio.h"
#include "../tty/tty.h"

typedef struct specifier
{
	size_t size;

	uint8_t flags;
	size_t width;
	size_t precision;
	uint8_t length;
	char type;
} specifier_t;

typedef struct handler
{
	char c;
	int (*f)(const specifier_t*, va_list*);
} handler_t;

static const char* next_specifier(const char* format, specifier_t* specifier)
{
	bzero(specifier, sizeof(specifier_t));

	while(*format && *format != '%') ++format;
	if(!(*format)) return (NULL);

	// TODO Parse the specifier
	specifier->size = 2;
	specifier->type = format[1];

	return format;
}

static inline int putchar(const char c)
{
	tty_write(&c, 1);
	return 1;
}

static inline int putint(int n, const size_t base)
{
	if(n < 0) {
		putchar('-');
		n = -n; // TODO OVERFLOW!
	}

	if((unsigned int) n >= base) {
		return putint(n / base, base) + putchar('0' + (n % base));
	} else {
		return putchar('0' + (n % base));
	}
}

static inline int putuint(unsigned int n, const size_t base)
{
	if(n >= base) {
		return putint(n / base, base) + putchar('0' + (n % base));
	} else {
		return putchar('0' + (n % base));
	}
}

static inline int putstr(const char* str)
{
	const size_t len = strlen(str);

	tty_write(str, len);
	return len;
}

static int signed_decimal(const specifier_t* specifier, va_list* args)
{
	// TODO Alignements, etc...
	(void) specifier;

	return putint(va_arg(*args, int), 10);
}

static int unsigned_decimal(const specifier_t* specifier, va_list* args)
{
	// TODO Alignements, etc...
	(void) specifier;

	return putuint(va_arg(*args, int), 10);
}

static int string(const specifier_t* specifier, va_list* args)
{
	// TODO Alignements, etc...
	(void) specifier;

	return putstr(va_arg(*args, const char*));
}

static int handle_specifier(const specifier_t* specifier, va_list* args)
{
	static handler_t handlers[] = {
		{'d', signed_decimal},
		{'i', signed_decimal},
		{'u', unsigned_decimal},
		{'s', string}
	};

	if(specifier->type == '%') {
		putchar('%');
		return 1;
	}

	for(size_t i = 0; i < sizeof(handlers) / sizeof(handler_t); ++i) {
		const handler_t* h = handlers + i;
		if(h->c == specifier->type) return h->f(specifier, args);
	}

	// TODO Do something?
	return 0;
}

int printf(const char* format, ...)
{
	int total = 0;
	va_list args;
	const char* s;
	specifier_t specifier;
	size_t len;

	va_start(args, format);

	while(*format) {
		s = next_specifier(format, &specifier);
		len	= (s ? (size_t) (s - format) : strlen(format));

		tty_write(format, len);
		format += len;
		total += len;

		if(s) total += handle_specifier(&specifier, &args);
		format += specifier.size;
	}

	va_end(args);
	return total;
}
