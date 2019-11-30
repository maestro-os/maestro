#include <acpi/aml/aml_parser.h>

aml_node_t *access_type(aml_parse_context_t *context)
{
	return parse_node(AML_ACCESS_TYPE, context, 1, byte_data);
}

aml_node_t *access_attrib(aml_parse_context_t *context)
{
	return parse_node(AML_ACCESS_ATTRIB, context, 1, byte_data);
}

aml_node_t *extended_access_attrib(aml_parse_context_t *context)
{
	return parse_node(AML_EXTENDED_ACCESS_ATTRIB, context, 1, byte_data);
}

aml_node_t *access_length(aml_parse_context_t *context)
{
	// TODO
	(void) context;
	return NULL;
}
