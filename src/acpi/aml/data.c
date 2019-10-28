#include <acpi/aml/aml_parser.h>

static aml_node_t *byte_const(const char **src, size_t *len)
{
	const char *s;
	size_t l;
	aml_node_t *node;

	if(*len < 2 || **src != BYTE_PREFIX)
		return NULL;
	s = (*src)++;
	l = (*len)--;
	if(!(node = parse_node(AML_BYTE_CONST, src, len, 1, byte_data)))
	{
		*src = s;
		*len = l;
	}
	return node;
}

static aml_node_t *word_const(const char **src, size_t *len)
{
	const char *s;
	size_t l;
	aml_node_t *node;

	if(*len < 3 || **src != WORD_PREFIX)
		return NULL;
	s = (*src)++;
	l = (*len)--;
	if(!(node = parse_node(AML_WORD_CONST, src, len, 1, word_data)))
	{
		*src = s;
		*len = l;
	}
	return node;
}

static aml_node_t *dword_const(const char **src, size_t *len)
{
	const char *s;
	size_t l;
	aml_node_t *node;

	if(*len < 5 || **src != DWORD_PREFIX)
		return NULL;
	s = (*src)++;
	l = (*len)--;
	if(!(node = parse_node(AML_DWORD_CONST, src, len, 1, dword_data)))
	{
		*src = s;
		*len = l;
	}
	return node;
}

static aml_node_t *qword_const(const char **src, size_t *len)
{
	const char *s;
	size_t l;
	aml_node_t *node;

	if(*len < 9 || **src != QWORD_PREFIX)
		return NULL;
	s = (*src)++;
	l = (*len)--;
	if(!(node = parse_node(AML_QWORD_CONST, src, len, 1, qword_data)))
	{
		*src = s;
		*len = l;
	}
	return node;
}

static aml_node_t *const_obj(const char **src, size_t *len)
{
	const char *s;
	size_t l;
	aml_node_t *node;

	if(*len < 1 || (**src != ZERO_OP && **src != ONE_OP && **src != ONES_OP))
		return NULL;
	s = (*src)++;
	l = (*len)--;
	if(!(node = node_new(AML_CONST_OBJ, *src, 1)))
	{
		*src = s;
		*len = l;
	}
	return node;
}

static aml_node_t *revision_op(const char **src, size_t *len)
{
	aml_node_t *node;

	if(*len < 2 || (*src)[0] != EXT_OP_PREFIX || (*src)[1] != REVISION_OP)
		return NULL;
	if((node = node_new(AML_REVISION_OP, *src, 2)))
	{
		*src += 2;
		*len -= 2;
	}
	return node;
}

static aml_node_t *computational_data(const char **src, size_t *len)
{
	return parse_either(AML_COMPUTATIONAL_DATA, src, len,
		8, byte_const, word_const, dword_const, qword_const,
			string, const_obj, revision_op, def_buffer);
}

aml_node_t *data_object(const char **src, size_t *len)
{
	return parse_either(AML_DATA_OBJECT, src, len,
		3, computational_data, def_package, def_var_package);
}

aml_node_t *byte_data(const char **src, size_t *len)
{
	aml_node_t *node;

	if(*len < 1 || !(node = node_new(AML_BYTE_DATA, *src, 1)))
		return NULL;
	++(*src);
	--(*len);
	return node;
}

aml_node_t *word_data(const char **src, size_t *len)
{
	return parse_node(AML_WORD_DATA, src, len, 2, byte_data, byte_data);
}

aml_node_t *dword_data(const char **src, size_t *len)
{
	return parse_node(AML_DWORD_DATA, src, len, 2, word_data, word_data);
}

aml_node_t *qword_data(const char **src, size_t *len)
{
	return parse_node(AML_QWORD_DATA, src, len, 2, dword_data, dword_data);
}
