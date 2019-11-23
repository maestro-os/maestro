#include <acpi/aml/aml_parser.h>

aml_node_t *field_flags(blob_t *blob)
{
	return parse_node(AML_FIELD_FLAGS, blob, 1, byte_data);
}

static aml_node_t *named_field(blob_t *blob)
{
	return parse_node(AML_NAMED_FIELD, blob, 2, name_seg, pkg_length);
}

static aml_node_t *reserved_field(blob_t *blob)
{
	blob_t b;
	aml_node_t *node;

	BLOB_COPY(blob, &b);
	if(!BLOB_CHECK(blob, 0x0))
		return NULL;
	if(!(node = parse_node(AML_RESERVED_FIELD, blob, 1, pkg_length)))
		BLOB_COPY(&b, blob);
	return node;
}

static aml_node_t *access_field(blob_t *blob)
{
	blob_t b;
	aml_node_t *node;

	BLOB_COPY(blob, &b);
	if(!BLOB_CHECK(blob, 0x01))
		return NULL;
	if(!(node = parse_node(AML_ACCESS_FIELD, blob,
		2, access_type, access_attrib)))
		BLOB_COPY(&b, blob);
	return node;
}

static aml_node_t *extended_access_field(blob_t *blob)
{
	blob_t b;
	aml_node_t *node;

	BLOB_COPY(blob, &b);
	if(!BLOB_CHECK(blob, 0x03))
		return NULL;
	if(!(node = parse_node(AML_ACCESS_FIELD, blob,
		3, access_type, extended_access_attrib, access_length)))
		BLOB_COPY(&b, blob);
	return node;
}

static aml_node_t *connect_field(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

static aml_node_t *field_element(blob_t *blob)
{
	return parse_either(AML_FIELD_ELEMENT, blob,
		5, named_field, reserved_field, access_field,
			extended_access_field, connect_field);
}

aml_node_t *field_list(blob_t *blob)
{
	return parse_list(AML_FIELD_LIST, blob, field_element);
}
