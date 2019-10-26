#include <acpi/aml/aml_parser.h>

uint8_t aml_get_byte(aml_node_t *node)
{
	if(!node)
		return 0;
	return (uint8_t) *node->data;
}

uint16_t aml_get_word(aml_node_t *node)
{
	if(!node)
		return 0;
	return ((uint16_t) aml_get_byte(node->children) << 8)
		| aml_get_byte(node->children->next);
}

uint32_t aml_get_dword(aml_node_t *node)
{
	if(!node)
		return 0;
	return ((uint32_t) aml_get_word(node->children) << 16)
		| aml_get_word(node->children->next);
}
