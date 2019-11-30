#include <acpi/aml/aml_parser.h>

aml_node_t *def_bank_field(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *node;

	BLOB_COPY(context, &c);
	if(!BLOB_CHECK(context, EXT_OP_PREFIX) || !BLOB_CHECK(context, BANK_FIELD_OP))
	{
		BLOB_COPY(&c, context);
		return NULL;
	}
	if(!(node = parse_explicit(AML_DEF_BANK_FIELD, context,
		6, pkg_length, name_string, name_string, bank_value,
			field_flags, field_list)))
		BLOB_COPY(&c, context);
	return node;
}

aml_node_t *bank_value(aml_parse_context_t *context)
{
	// TODO
	(void) context;
	return NULL;
}
