#include <acpi/aml/aml_parser.h>

static aml_node_t *object(aml_parse_context_t *context)
{
	return parse_either(AML_OBJECT, context,
		2, namespace_modifier_obj, named_obj);
}

static aml_node_t *term_obj(aml_parse_context_t *context)
{
	printf("term_obj: (remaining: %u)\n", (unsigned) context->len);
	print_memory(context->src, 16);
	return parse_either(AML_TERM_OBJ, context,
		3, object, type1_opcode, type2_opcode);
}

aml_node_t *term_list(aml_parse_context_t *context)
{
	return parse_list(AML_TERM_LIST, context, term_obj);
}

aml_node_t *term_arg(aml_parse_context_t *context)
{
	printf("term_arg (remaining: %u)\n", (unsigned) context->len);
	print_memory(context->src, 16);
	return parse_either(AML_TERM_ARG, context,
		4, type2_opcode, data_object, arg_obj, local_obj);
}

static aml_node_t *aml_code(aml_parse_context_t *context)
{
	return parse_node(AML_CODE, context, 1, term_list);
}

aml_node_t *aml_parse(const char *src, const size_t len)
{
	aml_parse_context_t context;
	aml_node_t *n;

	if(!src || len == 0)
		return NULL;
	context.decl = 1;
	context.methods = NULL;
	context.src = src;
	context.len = len;
	ast_free(aml_code(&context));
	context.decl = 0;
	context.src = src;
	context.len = len;
	n = aml_code(&context);
	aml_method_free(context.methods);
	return n;
}
