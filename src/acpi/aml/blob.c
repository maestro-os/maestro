#include <acpi/aml/aml_parser.h>

int blob_check(aml_parse_context_t *context, const char c)
{
	if(context->len == 0 || context->src[0] != c)
		return 0;
	BLOB_CONSUME(context, 1);
	return 1;
}
