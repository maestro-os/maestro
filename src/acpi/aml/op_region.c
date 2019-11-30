#include <acpi/aml/aml_parser.h>

static aml_node_t *region_space(aml_parse_context_t *context)
{
	return parse_node(AML_REGION_SPACE, context, 1, byte_data);
}

static aml_node_t *region_offset(aml_parse_context_t *context)
{
	return parse_node(AML_REGION_OFFSET, context, 1, term_arg);
}

static aml_node_t *region_len(aml_parse_context_t *context)
{
	return parse_node(AML_REGION_LEN, context, 1, term_arg);
}

aml_node_t *def_op_region(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *node;

	BLOB_COPY(context, &c);
	if(!BLOB_CHECK(context, EXT_OP_PREFIX)
		|| !BLOB_CHECK(context, OP_REGION_OP))
	{
		BLOB_COPY(&c, context);
		return NULL;
	}
	if(!(node = parse_node(AML_DEF_OP_REGION, context,
		4, name_string, region_space, region_offset, region_len)))
		BLOB_COPY(&c, context);
	return node;
}
