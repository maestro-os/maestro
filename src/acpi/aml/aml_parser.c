#include <acpi/aml/aml_parser.h>

static aml_node_t *object(const char **src, size_t *len)
{
	return parse_either(AML_OBJECT, src, len,
		2, namespace_modifier_obj, named_obj);
}

static aml_node_t *term_obj(const char **src, size_t *len)
{
	printf("term_obj:\n");
	print_memory(*src, 16);
	return parse_either(AML_TERM_OBJ, src, len,
		3, object, type1_opcode, type2_opcode);
}

aml_node_t *term_list(const char **src, size_t *len)
{
	printf("term_list:\n");
	print_memory(*src, 16);
	return parse_list(AML_TERM_LIST, src, len, term_obj);
}

aml_node_t *term_arg(const char **src, size_t *len)
{
	printf("term_arg\n");
	print_memory(*src, 16);
	return parse_either(AML_TERM_ARG, src, len,
		4, type2_opcode, data_object, arg_obj, local_obj);
}

static aml_node_t *aml_code(const char **src, size_t *len)
{
	return parse_node(AML_CODE, src, len, 1, term_list);
}

aml_node_t *aml_parse(const char *src, size_t len)
{
	if(!src || !len || len == 0)
		return NULL;
	return aml_code(&src, &len);
}
