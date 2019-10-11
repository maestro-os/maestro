#include <acpi/aml/aml_parser.h>

static aml_node_t *pkg_lead_byte(const char **src, size_t *len, int *n)
{
	aml_node_t *node;

	if(*len < 1 || !(node = node_new(PKG_LEAD_BYTE, *src, 1)))
		return NULL;
	*n = (**src >> 6) & 0b11;
	++(*src);
	--(*len);
	return node;
}

aml_node_t *pkg_length(const char **src, size_t *len)
{
	const char *s;
	size_t l;
	aml_node_t *node, *child;
	int i = 0, n;

	if(!(node = node_new(PKG_LENGTH, NULL, 0)))
		return NULL;
	s = *src;
	l = *len;
	if(!(node->children = pkg_lead_byte(src, len, &n)))
		goto fail;
	while(i++ < n)
	{
		if(!(child = byte_data(src, len)))
			goto fail;
		node_add_child(node, child);
	}
	return node;

fail:
	ast_free(node);
	*src = s;
	*len = l;
	return NULL;
}
