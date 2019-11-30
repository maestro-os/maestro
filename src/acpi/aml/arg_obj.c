#include <acpi/aml/aml_parser.h>

aml_node_t *arg_obj(aml_parse_context_t *context)
{
	aml_node_t *node;

	if(BLOB_EMPTY(context) || !IS_ARG_OP(BLOB_PEEK(context)))
		return NULL;
	if((node = node_new(AML_ARG_OBJ, &BLOB_PEEK(context), 1)))
		BLOB_CONSUME(context, 1);
	return node;
}

aml_node_t *local_obj(aml_parse_context_t *context)
{
	aml_node_t *node;

	if(BLOB_EMPTY(context) || !IS_LOCAL_OP(BLOB_PEEK(context)))
		return NULL;
	if((node = node_new(AML_LOCAL_OBJ, &BLOB_PEEK(context), 1)))
		BLOB_CONSUME(context, 1);
	return node;
}
