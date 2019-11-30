#include <acpi/aml/aml_parser.h>

aml_node_t *field_flags(aml_parse_context_t *context)
{
	return parse_node(AML_FIELD_FLAGS, context, 1, byte_data);
}

static aml_node_t *named_field(aml_parse_context_t *context)
{
	return parse_node(AML_NAMED_FIELD, context, 2, name_seg, pkg_length);
}

static aml_node_t *reserved_field(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *node;

	BLOB_COPY(context, &c);
	if(!BLOB_CHECK(context, 0x0))
		return NULL;
	if(!(node = parse_node(AML_RESERVED_FIELD, context, 1, pkg_length)))
		BLOB_COPY(&c, context);
	return node;
}

static aml_node_t *access_field(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *node;

	BLOB_COPY(context, &c);
	if(!BLOB_CHECK(context, 0x01))
		return NULL;
	if(!(node = parse_node(AML_ACCESS_FIELD, context,
		2, access_type, access_attrib)))
		BLOB_COPY(&c, context);
	return node;
}

static aml_node_t *extended_access_field(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *node;

	BLOB_COPY(context, &c);
	if(!BLOB_CHECK(context, 0x03))
		return NULL;
	if(!(node = parse_node(AML_ACCESS_FIELD, context,
		3, access_type, extended_access_attrib, access_length)))
		BLOB_COPY(&c, context);
	return node;
}

static aml_node_t *connect_field(aml_parse_context_t *context)
{
	// TODO
	(void) context;
	return NULL;
}

static aml_node_t *field_element(aml_parse_context_t *context)
{
	return parse_either(AML_FIELD_ELEMENT, context,
		5, named_field, reserved_field, access_field,
			extended_access_field, connect_field);
}

aml_node_t *field_list(aml_parse_context_t *context)
{
	return parse_list(AML_FIELD_LIST, context, field_element);
}
