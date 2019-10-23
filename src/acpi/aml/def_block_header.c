#include <acpi/aml/aml_parser.h>

static aml_node_t *table_signature(const char **src, size_t *len)
{
	return parse_node(AML_TABLE_SIGNATURE, src, len, 1, dword_data);
}

static aml_node_t *table_length(const char **src, size_t *len)
{
	return parse_node(AML_TABLE_LENGTH, src, len, 1, dword_data);
}

static aml_node_t *spec_compliance(const char **src, size_t *len)
{
	return parse_node(AML_SPEC_COMPLIANCE, src, len, 1, byte_data);
}

static aml_node_t *checksum(const char **src, size_t *len)
{
	return parse_node(AML_CHECK_SUM, src, len, 1, byte_data);
}

static aml_node_t *OEM_id_(const char **src, size_t *len)
{
	return parse_string(src, len, 6, byte_data);
}

static aml_node_t *OEM_id(const char **src, size_t *len)
{
	return parse_node(AML_OEM_ID, src, len, 1, OEM_id_);
}

static aml_node_t *OEM_tableid_(const char **src, size_t *len)
{
	return parse_string(src, len, 8, byte_data);
}

static aml_node_t *OEM_tableid(const char **src, size_t *len)
{
	return parse_node(AML_OEM_TABLE_ID, src, len, 1, OEM_tableid_);
}

static aml_node_t *OEM_revision(const char **src, size_t *len)
{
	return parse_node(AML_OEM_REVISION, src, len, 1, dword_data);
}

static aml_node_t *creator_id(const char **src, size_t *len)
{
	return parse_node(AML_CREATOR_ID, src, len, 1, dword_data);
}

static aml_node_t *creator_revision(const char **src, size_t *len)
{
	return parse_node(AML_CREATOR_REVISION, src, len, 1, dword_data);
}

aml_node_t *def_block_header(const char **src, size_t *len)
{
	return parse_node(AML_DEF_BLOCK_HEADER, src, len,
		9, table_signature, table_length, spec_compliance, checksum,
			OEM_id, OEM_tableid, OEM_revision, creator_id, creator_revision);
}
