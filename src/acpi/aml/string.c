#include <acpi/aml/aml_parser.h>
#include <libc/ctype.h>

static size_t string_length(const char *src, const size_t len)
{
	size_t n = 0;

	while(src[n] && isascii(src[n]) && n < len)
		++n;
	return (src[n] ? 0 : n);
}

aml_node_t *string(blob_t *blob)
{
	blob_t b;
	size_t n;
	aml_node_t *node;

	BLOB_COPY(blob, &b);
	if(BLOB_REMAIN(blob) < 2 || !BLOB_CHECK(blob, STRING_PREFIX))
		return NULL;
	if((n = string_length(&BLOB_PEEK(blob), BLOB_REMAIN(blob))) == 0
		|| !(node = node_new(AML_STRING, &BLOB_PEEK(blob), n + 1)))
	{
		BLOB_COPY(&b, blob);
		return NULL;
	}
	return node;
}
