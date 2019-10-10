#include <acpi/aml/aml_parser.h>

static aml_node_t *named_obj(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

static aml_node_t *object(const char **src, size_t *len)
{
	return parse_either(src, len, 2, namespace_modifier_obj, named_obj);
}

static aml_node_t *term_obj(const char **src, size_t *len)
{
	return parse_either(src, len, 4, namespace_modifier_obj,
		object, type1_opcode, type2_opcode);
}

aml_node_t *term_list(const char **src, size_t *len)
{
	const char *s;
	size_t l;
	aml_node_t *node, *n, *children = NULL, *last_child = NULL;

	errno = 0;
	if(!(node = node_new(NULL, 0)))
		return NULL;
	s = *src;
	l = *len;
	// TODO Do in recursive
	while((n = term_obj(src, len)))
	{
		if(!last_child)
			last_child = children = n;
		else
		{
			last_child->next = n;
			last_child = n;
		}
	}
	node->children = children;
	if(errno)
	{
		ast_free(node);
		*src = s;
		*len = l;
		return NULL;
	}
	else
		return node;
}

static aml_node_t *aml_code(const char **src, size_t *len)
{
	return parse_node(src, len, 2, def_block_header, term_list);
}

aml_node_t *aml_parse(const char *src, size_t len)
{
	if(!src || !len || len == 0)
		return NULL;
	return aml_code(&src, &len);
}
