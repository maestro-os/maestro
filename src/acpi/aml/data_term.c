#include <acpi/aml/aml_parser.h>

static aml_node_t *ddb_handle(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *data_ref_object(blob_t *blob)
{
	return parse_either(AML_DATA_OBJECT, blob,
		3, data_object, obj_reference, ddb_handle);
}
