#include <acpi/aml/aml_parser.h>

static aml_node_t *def_alias(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *node;

	BLOB_COPY(context, &c);
	if(!BLOB_CHECK(context, ALIAS_OP))
		return NULL;
	if(!(node = parse_node(AML_DEF_ALIAS, context,
		2, name_string, name_string)))
		BLOB_COPY(&c, context);
	return node;
}

static aml_node_t *def_name(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *node;

	BLOB_COPY(context, &c);
	if(!BLOB_CHECK(context, NAME_OP))
		return NULL;
	if(!(node = parse_node(AML_DEF_NAME, context,
		2, name_string, data_ref_object)))
		BLOB_COPY(&c, context);
	return node;
}

static aml_node_t *def_scope(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *node;

	BLOB_COPY(context, &c);
	if(!BLOB_CHECK(context, SCOPE_OP))
		return NULL;
	if(!(node = parse_explicit(AML_DEF_SCOPE, context,
		3, pkg_length, name_string, term_list)))
		BLOB_COPY(&c, context);
	return node;
}

aml_node_t *namespace_modifier_obj(aml_parse_context_t *context)
{
	return parse_either(AML_NAME_SPACE_MODIFIER_OBJ, context,
		3, def_alias, def_name, def_scope);
}
