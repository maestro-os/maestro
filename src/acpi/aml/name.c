#include <acpi/aml/aml_parser.h>

static aml_node_t *root_char(const char **src, size_t *len)
{
	aml_node_t *node;

	if(*len < 1 || **src != '\\' || !(node = NEW_NODE()))
		return NULL;
	++(*src);
	--(*len);
	return node;
}

static aml_node_t *prefix_path(const char **src, size_t *len)
{
	aml_node_t *node;

	if(*len < 1 || **src != '^' || !(node = NEW_NODE))
		return NULL;
	++(*src);
	--(*len);
	node->children = prefix_path(src, len);
	return node;
}

aml_node_t *name_string(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}
