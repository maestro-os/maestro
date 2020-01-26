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

size_t aml_pkg_length_get(const aml_node_t *node)
{
	const aml_node_t *lead, *byte;
	size_t n, i = 0, len = 0;

	if(!node || node->type != AML_PKG_LENGTH)
		return 0;
	if(!(lead = node->children) || lead->type != AML_PKG_LEAD_BYTE)
		return 0;
	if((n = (lead->data[0] >> 6) & 0b11) == 0)
		return lead->data[0];
	byte = lead->next;
	while(i++ < n)
	{
		len = (len << 8) | (byte->data[0] & 0xff);
		byte = byte->next;
	}
	return (len << 4) | (lead->data[0] & 0b1111);
}

static size_t name_string_length(const aml_node_t *node)
{
	size_t i = 0;
	aml_node_t *n;

	if(!node->children)
		return i;
	if(node->children->type == AML_ROOT_CHAR)
		++i;
	else
	{
		n = node->children;
		while(n && n->type == AML_PREFIX_PATH)
		{
			++i;
			n = n->children;
		}
	}
	if(!node->children->next)
		return 0;
	n = node->children->next->children;
	while(n)
	{
		i += 4;
		n = n->next;
	}
	return i;
}

const char *aml_name_string_get(const aml_node_t *node)
{
	size_t i;
	char *str;
	aml_node_t *n;

	if(!node || node->type != AML_NAME_STRING || !node->children)
		return NULL;
	if((i = name_string_length(node)) == 0 || !(str = kmalloc(i + 1)))
		return NULL;
	str[i] = '\0';
	i = 0;
	if(node->children->type == AML_ROOT_CHAR)
		str[i++] = '\\';
	else
	{
		n = node->children;
		while(n && n->type == AML_PREFIX_PATH)
		{
			str[i++] = '^';
			n = n->children;
		}
	}
	n = node->children->next->children;
	while(n)
	{
		memcpy(str + i, n->data, 4);
		i += 4;
		n = n->next;
	}
	return str;
}
