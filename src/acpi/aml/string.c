#include <acpi/aml/aml_parser.h>
#include <libc/ctype.h>

static aml_node_t *ascii_char(blob_t *blob)
{
	aml_node_t *n;

	if(BLOB_EMPTY(blob) || BLOB_PEEK(blob) < 0x01 || BLOB_PEEK(blob) > 0x7f)
		return NULL;
	if((n = node_new(AML_ASCII_CHAR, &BLOB_PEEK(blob), 1)))
		BLOB_CONSUME(blob, 1);
	return n;
}

static aml_node_t *ascii_char_list(blob_t *blob)
{
	return parse_list(AML_ASCII_CHAR_LIST, blob, ascii_char);
}

static aml_node_t *null_char(blob_t *blob)
{
	aml_node_t *n;

	if(BLOB_EMPTY(blob) || BLOB_PEEK(blob))
		return NULL;
	if((n = node_new(AML_ASCII_CHAR, &BLOB_PEEK(blob), 1)))
		BLOB_CONSUME(blob, 1);
	return n;
}

aml_node_t *string(blob_t *blob)
{
	blob_t b;
	aml_node_t *node;

	BLOB_COPY(blob, &b);
	if(!BLOB_CHECK(blob, STRING_PREFIX))
		return NULL;
	if(!(node = parse_node(AML_STRING, blob, 2, ascii_char_list, null_char)))
		BLOB_COPY(&b, blob);
	return node;
}
