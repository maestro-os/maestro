#include <acpi/aml/aml_parser.h>

static aml_node_t *ddb_handle(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *data_ref_object(const char **src, size_t *len)
{
	return parse_either(AML_DATA_OBJECT, src, len, 3,
		data_object, obj_reference, ddb_handle);
}
