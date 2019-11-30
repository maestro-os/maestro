#include <acpi/aml/aml_parser.h>

static aml_node_t *pkg_lead_byte(aml_parse_context_t *context, int *n)
{
	aml_node_t *node;

	if(BLOB_EMPTY(context)
		|| !(node = node_new(AML_PKG_LEAD_BYTE, &BLOB_PEEK(context), 1)))
		return NULL;
	*n = (BLOB_PEEK(context) >> 6) & 0b11;
	BLOB_CONSUME(context, 1);
	return node;
}

aml_node_t *pkg_length(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *node, *child;
	int i = 0, n;

	if(!(node = node_new(AML_PKG_LENGTH, &BLOB_PEEK(context), 0)))
		return NULL;
	BLOB_COPY(context, &c);
	if(!(child = pkg_lead_byte(context, &n)))
		goto fail;
	node_add_child(node, child);
	while(i++ < n)
	{
		if(!(child = byte_data(context)))
			goto fail;
		node_add_child(node, child);
	}
	return node;

fail:
	BLOB_COPY(&c, context);
	ast_free(node);
	return NULL;
}
