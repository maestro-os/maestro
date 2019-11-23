#include <acpi/aml/aml_parser.h>

int blob_check(blob_t *blob, const char c)
{
	if(blob->len == 0 || blob->src[0] != c)
		return 0;
	BLOB_CONSUME(blob, 1);
	return 1;
}
