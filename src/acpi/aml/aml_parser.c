#include <acpi/aml/aml_parser.h>

static aml_node_t *term_list(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

static aml_node_t *table_signature(const char **src, size_t *len)
{
	return parse_node(src, len, 1, dword_data);
}

static aml_node_t *table_length(const char **src, size_t *len)
{
	return parse_node(src, len, 1, dword_data);
}

static aml_node_t *spec_compliance(const char **src, size_t *len)
{
	return parse_node(src, len, 1, byte_data);
}

static aml_node_t *checksum(const char **src, size_t *len)
{
	return parse_node(src, len, 1, byte_data);
}

static aml_node_t *OEM_id(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

static aml_node_t *OEM_tableid(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

static aml_node_t *OEM_revision(const char **src, size_t *len)
{
	return parse_node(src, len, 1, dword_data);
}

static aml_node_t *creator_id(const char **src, size_t *len)
{
	return parse_node(src, len, 1, dword_data);
}

static aml_node_t *creator_revision(const char **src, size_t *len)
{
	return parse_node(src, len, 1, dword_data);
}

static aml_node_t *def_block_header(const char **src, size_t *len)
{
	return parse_node(src, len, 9, table_signature, table_length,
		spec_compliance, checksum, OEM_id, OEM_tableid, OEM_revision,
			creator_id, creator_revision);
}

static aml_node_t *aml_code(const char **src, size_t *len)
{
	return parse_node(src, len, 2, def_block_header, term_list);
}

aml_node_t *aml_parse(const char *src, size_t len)
{
	if(!src || !len || len == 0)
		return NULL;
	return aml_code(&src, &len);
}
