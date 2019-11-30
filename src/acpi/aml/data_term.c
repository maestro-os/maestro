#include <acpi/aml/aml_parser.h>

static aml_node_t *ddb_handle(aml_parse_context_t *context)
{
	// TODO
	(void) context;
	return NULL;
}

aml_node_t *data_ref_object(aml_parse_context_t *context)
{
	return parse_either(AML_DATA_OBJECT, context,
		3, data_object, obj_reference, ddb_handle);
}
