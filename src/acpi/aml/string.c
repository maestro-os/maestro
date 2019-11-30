#include <acpi/aml/aml_parser.h>
#include <libc/ctype.h>

static aml_node_t *ascii_char(aml_parse_context_t *context)
{
	aml_node_t *n;

	if(BLOB_EMPTY(context) || BLOB_PEEK(context) < 0x01
		|| BLOB_PEEK(context) > 0x7f)
		return NULL;
	if((n = node_new(AML_ASCII_CHAR, &BLOB_PEEK(context), 1)))
		BLOB_CONSUME(context, 1);
	return n;
}

static aml_node_t *ascii_char_list(aml_parse_context_t *context)
{
	return parse_list(AML_ASCII_CHAR_LIST, context, ascii_char);
}

static aml_node_t *null_char(aml_parse_context_t *context)
{
	aml_node_t *n;

	if(BLOB_EMPTY(context) || BLOB_PEEK(context))
		return NULL;
	if((n = node_new(AML_ASCII_CHAR, &BLOB_PEEK(context), 1)))
		BLOB_CONSUME(context, 1);
	return n;
}

aml_node_t *string(aml_parse_context_t *context)
{
	aml_parse_context_t b;
	aml_node_t *node;

	BLOB_COPY(context, &b);
	if(!BLOB_CHECK(context, STRING_PREFIX))
		return NULL;
	if(!(node = parse_node(AML_STRING, context, 2, ascii_char_list, null_char)))
		BLOB_COPY(&b, context);
	return node;
}
