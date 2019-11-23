#include <acpi/aml/aml_parser.h>

static aml_node_t *region_space(blob_t *blob)
{
	return parse_node(AML_REGION_SPACE, blob, 1, byte_data);
}

static aml_node_t *region_offset(blob_t *blob)
{
	return parse_node(AML_REGION_OFFSET, blob, 1, term_arg);
}

static aml_node_t *region_len(blob_t *blob)
{
	return parse_node(AML_REGION_LEN, blob, 1, term_arg);
}

aml_node_t *def_op_region(blob_t *blob)
{
	blob_t b;
	aml_node_t *node;

	BLOB_COPY(blob, &b);
	if(!BLOB_CHECK(blob, EXT_OP_PREFIX) || !BLOB_CHECK(blob, OP_REGION_OP))
	{
		BLOB_COPY(&b, blob);
		return NULL;
	}
	if(!(node = parse_node(AML_DEF_OP_REGION, blob,
		4, name_string, region_space, region_offset, region_len)))
		BLOB_COPY(&b, blob);
	return node;
}
