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

static aml_node_t *region_space(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

static aml_node_t *region_offset(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

static aml_node_t *region_len(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

static aml_node_t *def_op_region(const char **src, size_t *len)
{
	const char *s;
	size_t l;
	aml_node_t *node;

	if(*len < 2 || (*src)[0] != EXT_OP_PREFIX
		|| (uint8_t) (*src)[1] != OP_REGION_OP)
		return NULL;
	s = *src;
	l = *len;
	*src += 2;
	*len -= 2;
	if(!(node = parse_node(AML_DEF_OP_REGION, src, len,
		3, region_space, region_offset, region_len)))
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

#include <debug/debug.h> // TODO rm

aml_node_t *named_obj(const char **src, size_t *len)
{
	print_memory(*src, 16);
	return parse_either(AML_NAMED_OBJ, src, len,
		13, def_bank_field, def_create_bit_field, def_create_byte_field,
			def_create_dword_field, def_create_field, def_create_qword_field,
				def_create_word_field, def_data_region, def_external,
					def_op_region, def_power_res, def_processor,
						def_thermal_zone);
}
