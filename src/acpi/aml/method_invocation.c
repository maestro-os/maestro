#include "aml_parser.h"

static size_t count_args(const char *src, size_t len)
{
	// TODO
	(void) src;
	(void) len;
	return 0;
}

aml_node_t *method_invocation(const char **src, size_t *len)
{
	const char *s;
	size_t l;
	aml_node_t *node, *args;
	size_t args_count;

	if(!(node = parse_node(AML_METHOD_INVOCATION, src, len, 1, name_string)))
		return NULL;
	s = *src;
	l = *len;
	if((args_count = count_args(*src, *len)) == 0)
		return node;
	if(!(args = parse_fixed_list(AML_TERM_ARG_LIST, src, len,
		term_arg, args_count)))
	{
		*src = s;
		*len = l;
		ast_free(node);
		return NULL;
	}
	node_add_child(node, args);
	return node;
}
