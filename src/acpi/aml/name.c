#include <acpi/aml/aml_parser.h>

static aml_node_t *root_char(const char **src, size_t *len)
{
	aml_node_t *node;

	if(*len < 1 || IS_ROOT_CHAR(**src)
		|| !(node = node_new(AML_ROOT_CHAR, *src, 1)))
		return NULL;
	++(*src);
	--(*len);
	return node;
}

static aml_node_t *prefix_path(const char **src, size_t *len)
{
	aml_node_t *node;

	if(*len < 1 || IS_PREFIX_CHAR(**src)
		|| !(node = node_new(AML_PREFIX_PATH, *src, 1)))
		return NULL;
	++(*src);
	--(*len);
	node->children = prefix_path(src, len);
	return node;
}

aml_node_t *name_seg(const char **src, size_t *len)
{
	char buff[4];
	size_t i = 1;

	if(*len < 1 || !IS_LEAD_NAME_CHAR(**src))
		return NULL;
	memset(buff, '_', sizeof(buff));
	buff[0] = **src;
	++(*src);
	--(*len);
	while(i < 4 && *len > 0 && IS_NAME_CHAR(**src))
	{
		buff[i++] = **src;
		++(*src);
		--(*len);
	}
	return node_new(AML_NAME_SEG, buff, sizeof(buff));
}

static aml_node_t *dual_name_path(const char **src, size_t *len)
{
	const char *s;
	size_t l;
	aml_node_t *c0, *c1, *node;

	if(*len < 1 || **src != DUAL_NAME_PREFIX)
		return NULL;
	s = *src;
	l = *len;
	if(!(c0 = name_seg(src, len)) || !(c1 = name_seg(src, len))
		|| !(node = node_new(DUAL_NAME_PREFIX, NULL, 0)))
	{
		*src = s;
		*len = l;
		return NULL;
	}
	node_add_child(node, c0);
	node_add_child(node, c1);
	return node;
}

// TODO Clean
static aml_node_t *multi_name_path(const char **src, size_t *len)
{
	const char *s;
	size_t l;
	aml_node_t *c, *node;
	size_t i = 0, n;

	if(*len < 2 || **src != MULTI_NAME_PREFIX
		|| !(node = node_new(MULTI_NAME_PREFIX, NULL, 0)))
		return NULL;
	s = *src;
	l = *len;
	++(*src);
	--(*len);
	n = **src;
	++(*src);
	--(*len);
	while(i++ < n && (c = name_seg(src, len)))
		node_add_child(node, c);
	if(!c)
	{
		*src = s;
		*len = l;
		ast_free(node);
		return NULL;
	}
	return node;
}

static aml_node_t *null_name(const char **src, size_t *len)
{
	if(*len < 1 || **src)
		return NULL;
	return node_new(AML_NULL_NAME, *src, *len);
}

static aml_node_t *name_path(const char **src, size_t *len)
{
	// TODO Make a node?
	return parse_either(src, len,
		4, name_seg, dual_name_path, multi_name_path, null_name);
}

aml_node_t *name_string(const char **src, size_t *len)
{
	aml_node_t *node;
	const char *s;
	size_t l;

	if(!(node = node_new(AML_NAME_STRING, NULL, 0)))
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
