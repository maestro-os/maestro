#include <acpi/aml/aml_parser.h>

aml_node_t *def_bank_field(const char **src, size_t *len)
{
	const char *s;
	size_t l;
	aml_node_t *node;

	if(*len < 2 || (*src)[0] != EXT_OP_PREFIX || (*src)[1] != BANK_FIELD_OP)
		return NULL;
	s = *src;
	l = *len;
	*src += 2;
	*len -= 2;
	if(!(node = parse_node(AML_DEF_BANK_FIELD, src, len,
		6, pkg_length, name_string, name_string, bank_value,
			field_flags, field_list)))
	{
		*src = s;
		*len = l;
		return NULL;
	}
	return node;
}

aml_node_t *bank_value(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}
