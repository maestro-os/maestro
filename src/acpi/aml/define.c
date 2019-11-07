#include <acpi/aml/aml_parser.h>

static aml_node_t *def_alias(const char **src, size_t *len)
{
	const char *s;
	size_t l;
	aml_node_t *node;

	if(*len < 1 || **src != ALIAS_OP)
		return NULL;
	s = (*src)++;
	l = (*len)--;
	if(!(node = parse_node(AML_DEF_ALIAS, src, len,
		2, name_string, name_string)))
	{
		*src = s;
		*len = l;
	}
	return node;
}

static aml_node_t *def_name(const char **src, size_t *len)
{
	const char *s;
	size_t l;
	aml_node_t *node;

	if(*len < 1 || **src != NAME_OP)
		return NULL;
	s = (*src)++;
	l = (*len)--;
	if(!(node = parse_node(AML_DEF_NAME, src, len,
		2, name_string, data_ref_object)))
	{
		*src = s;
		*len = l;
	}
	return node;
}

static aml_node_t *def_scope(const char **src, size_t *len)
{
	const char *s;
	size_t l;
	aml_node_t *node;

	if(*len < 1 || **src != SCOPE_OP)
		return NULL;
	s = (*src)++;
	l = (*len)--;
	if(!(node = parse_node(AML_DEF_SCOPE, src, len,
		3, pkg_length, name_string, term_list)))
	{
		*src = s;
		*len = l;
	}
	return node;
}

aml_node_t *namespace_modifier_obj(const char **src, size_t *len)
{
	return parse_either(AML_NAME_SPACE_MODIFIER_OBJ, src, len,
		3, def_alias, def_name, def_scope);
}
