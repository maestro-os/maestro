#include "aml_parser.h"

aml_node_t *debug_obj(const char **src, size_t *len)
{
	const char *s;
	size_t l;
	aml_node_t *node;

	if(*len < 2 || (*src)[0] != EXT_OP_PREFIX || (*src)[1] != DEBUG_OP)
		return NULL;
	s = *src;
	l = *len;
	*src += 2;
	*len -= 2;
	if(!(node = node_new(AML_DEBUG_OBJ, *src, 0)))
	{
		*src = s;
		*len = l;
		return NULL;
	}
	return node;
}
