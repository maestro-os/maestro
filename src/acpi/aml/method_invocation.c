#include "aml_parser.h"

static size_t count_args(const char *src, size_t len)
{
	// TODO
	(void) src;
	(void) len;
	return 0;
}

aml_node_t *method_invocation(blob_t *blob)
{
	blob_t b;
	aml_node_t *node, *args;
	size_t args_count;

	if(!(node = parse_node(AML_METHOD_INVOCATION, blob, 1, name_string)))
		return NULL;
	BLOB_COPY(blob, &b);
	if((args_count = count_args(&BLOB_PEEK(blob), BLOB_REMAIN(blob))) == 0)
		return node;
	if(!(args = parse_fixed_list(AML_TERM_ARG_LIST, blob,
		term_arg, args_count)))
	{
		BLOB_COPY(&b, blob);
		ast_free(node);
		return NULL;
	}
	node_add_child(node, args);
	return node;
}
