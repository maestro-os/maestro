#include <acpi/aml/aml_parser.h>

static aml_node_t *def_alias(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

static aml_node_t *def_name(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

static aml_node_t *def_scope(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *namespace_modifier_obj(const char **src, size_t *len)
{
	return parse_either(src, len, 3, def_alias, def_name, def_scope);
}
