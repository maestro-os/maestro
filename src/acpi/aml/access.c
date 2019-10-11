#include <acpi/aml/aml_parser.h>

aml_node_t *access_type(const char **src, size_t *len)
{
	return parse_node(ACCESS_TYPE, src, len, 1, byte_data);
}

aml_node_t *access_attrib(const char **src, size_t *len)
{
	return parse_node(ACCESS_ATTRIB, src, len, 1, byte_data);
}

aml_node_t *extended_access_attrib(const char **src, size_t *len)
{
	return parse_node(EXTENDED_ACCESS_ATTRIB, src, len, 1, byte_data);
}

aml_node_t *access_length(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}
