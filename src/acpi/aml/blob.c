#include <acpi/aml/aml_parser.h>

void blob_consume(aml_parse_context_t *context, const size_t n)
{
	if(!context)
		return;
	context->src += n;
	context->len -= n;
}

int blob_check(aml_parse_context_t *context, const char c)
{
	if(context->len == 0 || context->src[0] != c)
		return 0;
	BLOB_CONSUME(context, 1);
	return 1;
}
