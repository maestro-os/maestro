#include <acpi/aml/aml_parser.h>

static aml_node_t *object(blob_t *blob)
{
	return parse_either(AML_OBJECT, blob, 2, namespace_modifier_obj, named_obj);
}

static aml_node_t *term_obj(blob_t *blob)
{
	printf("term_obj: (remaining: %u)\n", (unsigned) blob->len);
	print_memory(blob->src, 16);
	return parse_either(AML_TERM_OBJ, blob,
		3, object, type1_opcode, type2_opcode);
}

aml_node_t *term_list(blob_t *blob)
{
	return parse_list(AML_TERM_LIST, blob, term_obj);
}

aml_node_t *term_arg(blob_t *blob)
{
	printf("term_arg (remaining: %u)\n", (unsigned) blob->len);
	print_memory(blob->src, 16);
	return parse_either(AML_TERM_ARG, blob,
		4, type2_opcode, data_object, arg_obj, local_obj);
}

static aml_node_t *aml_code(blob_t *blob)
{
	return parse_node(AML_CODE, blob, 1, term_list);
}

aml_node_t *aml_parse(blob_t *blob)
{
	if(!blob || !blob->src || blob->len == 0)
		return NULL;
	return aml_code(blob);
}
