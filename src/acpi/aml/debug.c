#include "aml_parser.h"

aml_node_t *debug_obj(blob_t *blob)
{
	blob_t b;
	aml_node_t *node;

	BLOB_COPY(blob, &b);
	if(!BLOB_CHECK(blob, EXT_OP_PREFIX) || !BLOB_CHECK(blob, DEBUG_OP))
		return NULL;
	if(!(node = node_new(AML_DEBUG_OBJ, &BLOB_PEEK(blob), 0)))
		BLOB_COPY(&b, blob);
	return node;
}
