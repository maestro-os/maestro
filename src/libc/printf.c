#include <libc/ctype.h>
#include <libc/stdio.h>
#include <libc/stdlib.h>
#include <tty/tty.h>

#define SHARP_FLAG	0b000001
#define ZERO_FLAG	0b000010
#define MINUS_FLAG	0b000100
#define SPACE_FLAG	0b001000
#define PLUS_FLAG	0b010000
#define QUOTE_FLAG	0b100000

// TODO Fix %ju
// TODO %i doesn't print - on negative

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

static void skip_nbr(const char **str)
{
	while(isdigit(**str))
		++(*str);
}

static int read_int(const char **str)
{
	int n = 0, prev_n;
	char neg = 0;

	if(**str == '+' || (neg = **str == '-'))
		++(*str);
	while(**str >= '0' && **str <= '9')
	{
		prev_n = n;
		n = (n * 10) + (**str - '0');
		if(n < prev_n)
		{
			skip_nbr(str);
			return 0;
		}
		++(*str);
	}
	return (neg ? -n : n);
}

static uint8_t parse_length_(const char **s)
{
	if(**s == 'h')
	{
		++(*s);
		if(**s == 'h')
		{
			++(*s);
			return sizeof(char);
		}
		return sizeof(short int);
	}
	if(**s == 'l')
	{
		++(*s);
		if(**s == 'l')
		{
			++(*s);
			return sizeof(long long int);
		}
		return sizeof(long int);
	}
	if(**s == 'L')
	{
		++(*s);
		return sizeof(long double);
	}
	if(**s == 'j')
	{
		++(*s);
		return sizeof(intmax_t);
	}
	if(**s == 'z')
	{
		++(*s);
		return sizeof(size_t);
	}
	if(**s == 't')
	{
		++(*s);
		return sizeof(ptrdiff_t);
	}
	return 0;
}

static uint8_t parse_length(const char **s)
{
	uint8_t val;

	val = parse_length_(s);
	if(**s == 'n')
	{
		++(*s);
		return sizeof(void *);
	}
	return val;
}

static const char *next_specifier(const char *format, va_list *args,
	specifier_t *specifier)
{
	const char *begin, *s;

	bzero(specifier, sizeof(specifier_t));
	if(!(begin = strchr(format, '%')))
		return NULL;
	s = begin + 1;
	if(*s == '%')
	{
		++s;
		specifier->type = '%';
		goto end;
	}
	while(1)
	{
		if(*s == '#')
		{
			specifier->flags |= SHARP_FLAG;
			++s;
			continue;
		}
		if(*s == '0')
		{
			specifier->flags |= ZERO_FLAG;
			++s;
			continue;
		}
		if(*s == '-')
		{
			specifier->flags |= MINUS_FLAG;
			++s;
			continue;
		}
		if(*s == ' ')
		{
			specifier->flags |= SPACE_FLAG;
			++s;
			continue;
		}
		if(*s == '+')
		{
			specifier->flags |= PLUS_FLAG;
			++s;
			continue;
		}
		if(*s == '\'')
		{
			specifier->flags |= QUOTE_FLAG;
			++s;
			continue;
		}
		break;
	}
	if(specifier->flags & MINUS_FLAG)
		specifier->flags &= ~ZERO_FLAG;
	if(specifier->flags & PLUS_FLAG)
		specifier->flags &= ~SPACE_FLAG;
	if(isdigit(*s))
	{
		specifier->parameter_width = 1;
		specifier->width = read_int(&s);
	}
	else if(*s == '*')
	{
		++s;
		specifier->parameter_width = 1;
		specifier->width = va_arg(*args, int);
	}
	if(*s == '.')
	{
		++s;
		specifier->parameter_precision = 1;
		if(*s == '*')
		{
			++s;
			specifier->precision = va_arg(*args, int);
		}
		else
			specifier->precision = read_int(&s);
	}
	specifier->length = parse_length(&s);
	specifier->type = *s++;

end:
	specifier->size = s - begin;
	return begin;
}

static inline intmax_t get_arg(va_list *args, const size_t length)
{
	return va_arg(*args, int32_t);
	(void) length;
	/*if(length == 0 || length >= 4)
		return va_arg(*args, int32_t);
	return va_arg(*args, int32_t) & ((1 << length * 8) - 1);*/
}

static inline size_t intlen(int i, const unsigned base)
{
	size_t n = 0;

	if(i == 0)
		return 1;
	if(i < 0)
		++n;
	while(i != 0)
	{
		++n;
		i /= base;
	}
	return n;
}

static inline size_t uintlen(unsigned i, const unsigned base)
{
	size_t n = 0;

	if(i == 0)
		return 1;
	while(i != 0)
	{
		++n;
		i /= base;
	}
	return n;
}

static inline int putchar(const char c)
{
	tty_write(&c, 1, current_tty);
	return 1;
}

static inline int putzeros(const size_t n)
{
	size_t i = 0;

	while(i++ < n)
		putchar('0');
	return n;
}

static inline int putstr(const char *str)
{
	size_t len;

	len = strlen(str);
	tty_write(str, len, current_tty);
	return len;
}

static int flags_number_handle(const specifier_t *specifier,
	const size_t len, const int positive,
		const int sign, const unsigned base, const int upper)
{
	int total = 0;

	// TODO All flags
	if(specifier->flags & PLUS_FLAG && sign)
	{
		if(positive)
			total += putchar('+');
		else
			total += putchar('-');
	}
	if(specifier->flags & SPACE_FLAG && positive)
		total += putchar(' ');
	if(specifier->flags & SHARP_FLAG && base == 16)
		total += putstr(upper ? "0X" : "0x");
	if(specifier->flags & ZERO_FLAG && specifier->width > (total + len))
		total += putzeros(specifier->width - (total + len));
	return total;
}

static inline char get_number_char(int n, const int upper)
{
	if(n < 0)
		n = -n;
	if(n < 10)
		return '0' + n;
	else
		return (upper ? 'A' : 'a') + n - 10;
}

static inline int putint(const specifier_t *specifier, int n)
{
	if(n >= 10 || n <= -10)
		return putint(specifier, n / 10)
			+ putchar(get_number_char(n % 10, 0));
	return putchar(get_number_char(n % 10, 0));
}

static inline int putuint(const specifier_t *specifier, const unsigned n,
	const unsigned base, const int upper)
{
	if(n >= base)
		return putuint(specifier, n / base, base, upper)
			+ putchar(get_number_char(n % base, upper));
	else
		return putchar(get_number_char(n % base, upper));
}

static int signed_decimal_handler(const specifier_t *specifier, va_list *args)
{
	int n;

	n = get_arg(args, specifier->length);
	return flags_number_handle(specifier, intlen(n, 10), (n >= 0), 1, 10, 0)
		+ putint(specifier, n);
}

static int unsigned_octal_handler(const specifier_t *specifier, va_list *args)
{
	unsigned n;

	n = get_arg(args, specifier->length);
	return flags_number_handle(specifier, uintlen(n, 8), 1, 0, 8, 0)
		+ putuint(specifier, n, 8, 0);
}

static int unsigned_decimal_handler(const specifier_t *specifier, va_list *args)
{
	unsigned n;

	n = get_arg(args, specifier->length);
	return flags_number_handle(specifier, uintlen(n, 10), 1, 0, 10, 0)
		+ putuint(specifier, n, 10, 0);
}

static int unsigned_hexadecimal_handler(const specifier_t *specifier,
	va_list *args)
{
	unsigned n;

	n = get_arg(args, specifier->length);
	return flags_number_handle(specifier, uintlen(n, 16), 1, 0, 16, 0)
		+ putuint(specifier, n, 16, 0);
}

static int unsigned_upper_hexadecimal_handler(const specifier_t *specifier,
	va_list *args)
{
	unsigned n;

	n = get_arg(args, specifier->length);
	return flags_number_handle(specifier, uintlen(n, 16), 1, 0, 16, 1)
		+ putuint(specifier, n, 16, 1);
}

static int char_handler(const specifier_t *specifier, va_list *args)
{
	// TODO All flags
	(void) specifier;
	return putchar(va_arg(*args, int));
}

static int string_handler(const specifier_t *specifier, va_list *args)
{
	const char *str;
	size_t len;

	str = va_arg(*args, const char *);
	if(specifier->parameter_precision)
		len = strnlen(str, specifier->precision);
	else
		len = strlen(str);
	tty_write(str, len, current_tty);
	return len;
}

static int pointer_handler(const specifier_t *specifier, va_list *args)
{
	specifier_t s;
	uintptr_t n;

	s = *specifier;
	s.flags |= SHARP_FLAG;
	n = get_arg(args, s.length);
	return flags_number_handle(&s, uintlen(n, 16), 1, 0, 16, 0)
		+ putuint(&s, n, 16, 0);
}

static int handle_specifier(const specifier_t *specifier, va_list *args,
	int total)
{
	static const struct
	{
		char c;
		int (*f)(const specifier_t *, va_list *);
	} handlers[] = {
		{'d', signed_decimal_handler},
		{'i', signed_decimal_handler},
		{'o', unsigned_octal_handler},
		{'u', unsigned_decimal_handler},
		{'x', unsigned_hexadecimal_handler},
		{'X', unsigned_upper_hexadecimal_handler},
		// TODO doubles/floats
		{'c', char_handler},
		{'s', string_handler},
		{'p', pointer_handler},
		// TODO C, S
	};
	size_t i = 0;

	if(specifier->type == '%')
	{
		putchar('%');
		return 1;
	}
	if(specifier->type == 'n')
	{
		*va_arg(*args, int *) = total;
		return 0;
	}
	while(i < sizeof(handlers) / sizeof(*handlers))
	{
		if(handlers[i].c == specifier->type)
			return handlers[i].f(specifier, args);
		++i;
	}
	// TODO Do something? (invalid format)
	return 0;
}

/*
 * Prints a message on the screen with the specified format and arguments.
 * The format determines the number, location and type of the variadic
 * arguments. Refer to the POSIX standard for further documentation.
 */
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
		s = next_specifier(format, &args, &specifier);
		len	= (s ? (size_t) (s - format) : strlen(format));
		tty_write(format, len, current_tty);
		format += len;
		total += len;
		if(s)
			total += handle_specifier(&specifier, &args, total);
		format += specifier.size;
	}
	va_end(args);
	return total;
}
