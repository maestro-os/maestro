#include <acpi/aml/aml_parser.h>

aml_node_t *access_type(blob_t *blob)
{
	return parse_node(AML_ACCESS_TYPE, blob, 1, byte_data);
}

aml_node_t *access_attrib(blob_t *blob)
{
	return parse_node(AML_ACCESS_ATTRIB, blob, 1, byte_data);
}

aml_node_t *extended_access_attrib(blob_t *blob)
{
	return parse_node(AML_EXTENDED_ACCESS_ATTRIB, blob, 1, byte_data);
}

aml_node_t *access_length(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}
