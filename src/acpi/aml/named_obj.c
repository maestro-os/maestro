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

static aml_node_t *def_op_region(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
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
	return parse_either(src, len, 13, def_bank_field, def_create_bit_field,
		def_create_byte_field, def_create_dword_field, def_create_field,
			def_create_qword_field, def_create_word_field, def_data_region,
				def_external, def_op_region, def_power_res, def_processor,
					def_thermal_zone);
}
