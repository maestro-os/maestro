#include "aml_parser.h"

static size_t count_args(const char *src, size_t len)
{
	// TODO
	(void) src;
	(void) len;
	return 0;
}

aml_node_t *method_invocation(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *node, *args;
	size_t args_count;

	if(!(node = parse_node(AML_METHOD_INVOCATION, context, 1, name_string)))
		return NULL;
	// TODO Check that method exists
	BLOB_COPY(context, &c);
	args_count = count_args(&BLOB_PEEK(context), BLOB_REMAIN(context));
	if(args_count == 0)
		return node;
	if(!(args = parse_fixed_list(AML_TERM_ARG_LIST, context,
		term_arg, args_count)))
	{
		BLOB_COPY(&c, context);
		ast_free(node);
		return NULL;
	}
	node_add_child(node, args);
	return node;
}
