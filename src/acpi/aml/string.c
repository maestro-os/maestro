#include <acpi/aml/aml_parser.h>
#include <libc/ctype.h>

static size_t string_length(const char *src, const size_t len)
{
	size_t n = 0;

	while(src[n] && isascii(src[n]) && n < len)
		++n;
	return (src[n] ? 0 : n);
}

aml_node_t *string(const char **src, size_t *len)
{
	const char *s;
	size_t l;
	size_t n;
	aml_node_t *node;

	if(*len < 2 || **src != STRING_PREFIX)
		return NULL;
	s = (*src)++;
	l = (*len)++;
	if((n = string_length(*src, *len)) == 0
		|| !(node = node_new(AML_STRING, *src, n + 1)))
	{
		*src = s;
		*len = l;
		return NULL;
	}
	return node;
}
