#include <acpi/aml/aml_parser.h>

static aml_node_t *root_char(const char **src, size_t *len)
{
	aml_node_t *node;

	if(*len < 1 || IS_ROOT_CHAR(**src) || !(node = node_new(*src, 1)))
		return NULL;
	++(*src);
	--(*len);
	return node;
}

static aml_node_t *prefix_path(const char **src, size_t *len)
{
	aml_node_t *node;

	if(*len < 1 || IS_PREFIX_CHAR(**src) || !(node = node_new(*src, 1)))
		return NULL;
	++(*src);
	--(*len);
	node->children = prefix_path(src, len);
	return node;
}

static aml_node_t *name_seg(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

static aml_node_t *dual_name_path(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

static aml_node_t *multi_name_path(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

static aml_node_t *null_name(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

static aml_node_t *name_path(const char **src, size_t *len)
{
	return parse_either(src, len, 4, name_seg, dual_name_path,
		multi_name_path, null_name);
}

aml_node_t *name_string(const char **src, size_t *len)
{
	aml_node_t *node;
	const char *s;
	size_t l;

	if(!(node = node_new(NULL, 0)))
		return NULL;
	s = *src;
	l = *len;
	if(!((node->children = root_char(src, len))
		|| (node->children = prefix_path(src, len))))
		goto fail;
	if(!(node->children->next = name_path(src, len)))
		goto fail;
	return node;

fail:
	*src = s;
	*len = l;
	ast_free(node);
	return NULL;
}
