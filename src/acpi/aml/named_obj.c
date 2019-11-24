#include <acpi/aml/aml_parser.h>

static aml_node_t *def_create_bit_field(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

static aml_node_t *def_create_byte_field(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

static aml_node_t *def_create_dword_field(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

static aml_node_t *def_create_field(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

static aml_node_t *def_create_qword_field(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

static aml_node_t *def_create_word_field(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

static aml_node_t *def_data_region(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

static aml_node_t *def_device(blob_t *blob)
{
	blob_t b;
	aml_node_t *node;

	BLOB_COPY(blob, &b);
	if(!BLOB_CHECK(blob, EXT_OP_PREFIX) || !BLOB_CHECK(blob, DEVICE_OP))
	{
		BLOB_COPY(&b, blob);
		return NULL;
	}
	if(!(node = parse_explicit(AML_DEF_DEVICE, blob,
		3, pkg_length, name_string, term_list)))
		BLOB_COPY(&b, blob);
	return node;
}

static aml_node_t *def_external(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

static aml_node_t *def_field(blob_t *blob)
{
	blob_t b;
	aml_node_t *node;

	BLOB_COPY(blob, &b);
	if(!BLOB_CHECK(blob, EXT_OP_PREFIX) || !BLOB_CHECK(blob, FIELD_OP))
	{
		BLOB_COPY(&b, blob);
		return NULL;
	}
	if(!(node = parse_explicit(AML_DEF_FIELD, blob,
		4, pkg_length, name_string, field_flags, field_list)))
		BLOB_COPY(&b, blob);
	return node;
}

static aml_node_t *method_flags(blob_t *blob)
{
	return parse_node(AML_METHOD_FLAGS, blob, 1, byte_data);
}

static aml_node_t *def_method(blob_t *blob)
{
	printf("def_method\n");
	print_memory(blob->src, 16);
	return parse_operation(0, METHOD_OP, AML_DEF_METHOD, blob,
		4, pkg_length, name_string, method_flags, term_list);
}

static aml_node_t *sync_flags(blob_t *blob)
{
	return parse_node(AML_SYNC_FLAGS, blob, 1, byte_data);
}

static aml_node_t *def_mutex(blob_t *blob)
{
	return parse_operation(1, MUTEX_OP, AML_DEF_MUTEX, blob,
		2, name_string, sync_flags);
}

static aml_node_t *def_power_res(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

static aml_node_t *def_processor(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

static aml_node_t *def_thermal_zone(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

// TODO Cleanup
aml_node_t *named_obj(blob_t *blob)
{
	return parse_either(AML_NAMED_OBJ, blob,
		14, def_bank_field, def_create_bit_field, def_create_byte_field,
			def_create_dword_field, def_create_field, def_create_qword_field,
				def_create_word_field, def_data_region, def_device,
					def_external, def_field, def_method, def_mutex,
						def_op_region, def_power_res, def_processor,
							def_thermal_zone);
}
