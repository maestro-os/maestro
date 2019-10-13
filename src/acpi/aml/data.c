#include <acpi/aml/aml_parser.h>

aml_node_t *byte_data(const char **src, size_t *len)
{
	aml_node_t *node;

	if(*len < 1 || !(node = node_new(BYTE_DATA, *src, 1)))
		return NULL;
	++(*src);
	--(*len);
	return node;
}

aml_node_t *word_data(const char **src, size_t *len)
{
	return parse_node(WORD_DATA, src, len, 2, byte_data, byte_data);
}

aml_node_t *dword_data(const char **src, size_t *len)
{
	return parse_node(DWORD_DATA, src, len, 2, word_data, word_data);
}

aml_node_t *qword_data(const char **src, size_t *len)
{
	return parse_node(QWORD_DATA, src, len, 2, dword_data, dword_data);
}
