#include <acpi/aml/aml_parser.h>

aml_node_t *arg_obj(const char **src, size_t *len)
{
	aml_node_t *node;

	if(*len < 1 || !IS_ARG_OP(**src))
		return NULL;
	if((node = node_new(AML_ARG_OBJ, *src, 1)))
	{
		++(*src);
		--(*len);
	}
	return node;
}

aml_node_t *local_obj(const char **src, size_t *len)
{
	aml_node_t *node;

	if(*len < 1 || !IS_LOCAL_OP(**src))
		return NULL;
	if((node = node_new(AML_LOCAL_OBJ, *src, 1)))
	{
		++(*src);
		--(*len);
	}
	return node;
}
