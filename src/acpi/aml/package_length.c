#include <acpi/aml/aml_parser.h>

static aml_node_t *pkg_lead_byte(blob_t *blob, int *n)
{
	aml_node_t *node;

	if(BLOB_EMPTY(blob)
		|| !(node = node_new(AML_PKG_LEAD_BYTE, &BLOB_PEEK(blob), 1)))
		return NULL;
	*n = (BLOB_PEEK(blob) >> 6) & 0b11;
	BLOB_CONSUME(blob, 1);
	return node;
}

aml_node_t *pkg_length(blob_t *blob)
{
	blob_t b;
	aml_node_t *node, *child;
	int i = 0, n;

	if(!(node = node_new(AML_PKG_LENGTH, &BLOB_PEEK(blob), 0)))
		return NULL;
	BLOB_COPY(blob, &b);
	if(!(child = pkg_lead_byte(blob, &n)))
		goto fail;
	node_add_child(node, child);
	while(i++ < n)
	{
		if(!(child = byte_data(blob)))
			goto fail;
		node_add_child(node, child);
	}
	return node;

fail:
	BLOB_COPY(&b, blob);
	ast_free(node);
	return NULL;
}
