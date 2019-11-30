#include "aml_parser.h"

aml_node_t *debug_obj(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *node;

	BLOB_COPY(context, &c);
	if(!BLOB_CHECK(context, EXT_OP_PREFIX) || !BLOB_CHECK(context, DEBUG_OP))
	{
		BLOB_COPY(&c, context);
		return NULL;
	}
	if(!(node = node_new(AML_DEBUG_OBJ, &BLOB_PEEK(context), 0)))
		BLOB_COPY(&c, context);
	return node;
}
