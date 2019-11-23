#include <acpi/aml/aml_parser.h>

aml_node_t *def_bank_field(blob_t *blob)
{
	blob_t b;
	aml_node_t *node;

	BLOB_COPY(blob, &b);
	if(!BLOB_CHECK(blob, EXT_OP_PREFIX) || !BLOB_CHECK(blob, BANK_FIELD_OP))
		return NULL;
	if(!(node = parse_explicit(AML_DEF_BANK_FIELD, blob,
		6, pkg_length, name_string, name_string, bank_value,
			field_flags, field_list)))
	{
		BLOB_COPY(&b, blob);
		return NULL;
	}
	return node;
}

aml_node_t *bank_value(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}
