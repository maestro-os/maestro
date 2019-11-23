#include <acpi/aml/aml_parser.h>

aml_node_t *arg_obj(blob_t *blob)
{
	aml_node_t *node;

	if(BLOB_EMPTY(blob) || !IS_ARG_OP(BLOB_PEEK(blob)))
		return NULL;
	if((node = node_new(AML_ARG_OBJ, &BLOB_PEEK(blob), 1)))
		BLOB_CONSUME(blob, 1);
	return node;
}

aml_node_t *local_obj(blob_t *blob)
{
	aml_node_t *node;

	if(BLOB_EMPTY(blob) || !IS_LOCAL_OP(BLOB_PEEK(blob)))
		return NULL;
	if((node = node_new(AML_LOCAL_OBJ, &BLOB_PEEK(blob), 1)))
		BLOB_CONSUME(blob, 1);
	return node;
}
