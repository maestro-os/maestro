#include <acpi/aml/aml_parser.h>

aml_node_t *field_flags(const char **src, size_t *len)
{
	return parse_node(AML_FIELD_FLAGS, src, len, 1, byte_data);
}

static aml_node_t *named_field(const char **src, size_t *len)
{
	return parse_node(AML_NAMED_FIELD, src, len, 2, name_seg, pkg_length);
}

static aml_node_t *reserved_field(const char **src, size_t *len)
{
	const char *s;
	size_t l;
	aml_node_t *node;

	if(*len < 1 || **src)
		return NULL;
	s = *src;
	l = *len;
	++(*src);
	--(*len);
	if(!(node = parse_node(AML_RESERVED_FIELD, src, len, 1, pkg_length)))
	{
		*src = s;
		*len = l;
		return NULL;
	}
	return node;
}

static aml_node_t *access_field(const char **src, size_t *len)
{
	const char *s;
	size_t l;
	aml_node_t *node;

	if(*len < 1 || **src != 0x01)
		return NULL;
	s = *src;
	l = *len;
	++(*src);
	--(*len);
	if(!(node = parse_node(AML_ACCESS_FIELD, src, len,
		2, access_type, access_attrib)))
	{
		*src = s;
		*len = l;
		return NULL;
	}
	return node;
}

static aml_node_t *extended_access_field(const char **src, size_t *len)
{
	const char *s;
	size_t l;
	aml_node_t *node;

	if(*len < 1 || **src != 0x03)
		return NULL;
	s = *src;
	l = *len;
	++(*src);
	--(*len);
	if(!(node = parse_node(AML_ACCESS_FIELD, src, len,
		3, access_type, extended_access_attrib, access_length)))
	{
		*src = s;
		*len = l;
		return NULL;
	}
	return node;
}

static aml_node_t *connect_field(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

static aml_node_t *field_element(const char **src, size_t *len)
{
	return parse_either(AML_FIELD_ELEMENT, src, len,
		5, named_field, reserved_field, access_field,
			extended_access_field, connect_field);
}

aml_node_t *field_list(const char **src, size_t *len)
{
	return parse_list(AML_FIELD_LIST, src, len, field_element);
}
