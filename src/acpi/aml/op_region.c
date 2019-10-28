#include <acpi/aml/aml_parser.h>

static aml_node_t *region_space(const char **src, size_t *len)
{
	return parse_node(AML_REGION_SPACE, src, len, 1, byte_data);
}

static aml_node_t *region_offset(const char **src, size_t *len)
{
	return parse_node(AML_REGION_OFFSET, src, len, 1, term_arg);
}

static aml_node_t *region_len(const char **src, size_t *len)
{
	return parse_node(AML_REGION_LEN, src, len, 1, term_arg);
}

aml_node_t *def_op_region(const char **src, size_t *len)
{
	const char *s;
	size_t l;
	aml_node_t *node;

	if(*len < 2 || (*src)[0] != EXT_OP_PREFIX || (*src)[1] != OP_REGION_OP)
		return NULL;
	s = *src;
	l = *len;
	*src += 2;
	*len -= 2;
	if(!(node = parse_node(AML_DEF_OP_REGION, src, len,
		4, name_string, region_space, region_offset, region_len)))
	{
		*src = s;
		*len = l;
	}
	return node;
}
