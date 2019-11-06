#include "aml_parser.h"

aml_node_t *aml_search(aml_node_t *node, const enum node_type type)
{
	aml_node_t *n;

	if(!node)
		return NULL;
	node = node->children;
	while(node)
	{
		if(node->type == type)
			return node;
		if((n = aml_search(node, type)))
			return n;
		node = node->next;
	}
	return NULL;
}

int aml_get_integer(aml_node_t *node)
{
	aml_node_t *n;

	if(!(n = aml_search(node, AML_COMPUTATIONAL_DATA)))
		return 0;
	n = n->children;
	if(n->type == AML_BYTE_CONST)
	{
		n = n->children;
		return *((int8_t *) n->data);
	}
	else if(n->type == AML_WORD_CONST)
	{
		n = n->children;
		return *((int16_t *) n->data);
	}
	else if(n->type == AML_DWORD_CONST)
	{
		n = n->children;
		return *((int32_t *) n->data);
	}
	else if(n->type == AML_QWORD_CONST)
	{
		n = n->children;
		return *((int64_t *) n->data);
	}
	return 0;
}
