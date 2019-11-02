#include <acpi/aml/aml_parser.h>

static aml_node_t *def_create_bit_field(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

static aml_node_t *def_create_byte_field(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

static aml_node_t *def_create_dword_field(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

static aml_node_t *def_create_field(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

static aml_node_t *def_create_qword_field(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

static aml_node_t *def_create_word_field(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

static aml_node_t *def_data_region(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

static aml_node_t *def_external(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

static aml_node_t *def_field(const char **src, size_t *len)
{
	const char *s;
	size_t l;
	aml_node_t *node;

	if(*len < 2 || (*src)[0] != EXT_OP_PREFIX || (*src)[1] != FIELD_OP)
		return NULL;
	s = *src;
	l = *len;
	*src += 2;
	*len -= 2;
	if(!(node = parse_node(AML_DEF_FIELD, src, len,
		4, pkg_length, name_string, field_flags, field_list)))
	{
		*src = s;
		*len = l;
	}
	return node;
}

static aml_node_t *method_flags(const char **src, size_t *len)
{
	return parse_node(AML_METHOD_FLAGS, src, len, 1, byte_data);
}

static aml_node_t *def_method(const char **src, size_t *len)
{
	const char *s;
	size_t l;
	aml_node_t *node;

	if(*len < 1 || **src != METHOD_OP)
		return NULL;
	s = (*src)++;
	l = (*len)--;
	if(!(node = parse_node(AML_DEF_METHOD, src, len,
		4, pkg_length, name_string, method_flags, term_list)))
	{
		*src = s;
		*len = l;
	}
	return node;
}

static aml_node_t *def_power_res(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

static aml_node_t *def_processor(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

static aml_node_t *def_thermal_zone(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *named_obj(const char **src, size_t *len)
{
	return parse_either(AML_NAMED_OBJ, src, len,
		13, def_bank_field, def_create_bit_field, def_create_byte_field,
			def_create_dword_field, def_create_field, def_create_qword_field,
				def_create_word_field, def_data_region, def_external, def_field,
					def_method, def_op_region, def_power_res, def_processor,
						def_thermal_zone);
}
