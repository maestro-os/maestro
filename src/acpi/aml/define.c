#include <acpi/aml/aml_parser.h>

static aml_node_t *def_alias(blob_t *blob)
{
	blob_t b;
	aml_node_t *node;

	BLOB_COPY(blob, &b);
	if(!BLOB_CHECK(blob, ALIAS_OP))
		return NULL;
	if(!(node = parse_node(AML_DEF_ALIAS, blob, 2, name_string, name_string)))
		BLOB_COPY(&b, blob);
	return node;
}

static aml_node_t *def_name(blob_t *blob)
{
	blob_t b;
	aml_node_t *node;

	BLOB_COPY(blob, &b);
	if(!BLOB_CHECK(blob, NAME_OP))
		return NULL;
	if(!(node = parse_node(AML_DEF_NAME, blob,
		2, name_string, data_ref_object)))
		BLOB_COPY(&b, blob);
	return node;
}

static aml_node_t *def_scope(blob_t *blob)
{
	blob_t b;
	aml_node_t *node;

	BLOB_COPY(blob, &b);
	if(!BLOB_CHECK(blob, SCOPE_OP))
		return NULL;
	if(!(node = parse_explicit(AML_DEF_SCOPE, blob,
		3, pkg_length, name_string, term_list)))
		BLOB_COPY(&b, blob);
	return node;
}

aml_node_t *namespace_modifier_obj(blob_t *blob)
{
	return parse_either(AML_NAME_SPACE_MODIFIER_OBJ, blob,
		3, def_alias, def_name, def_scope);
}
