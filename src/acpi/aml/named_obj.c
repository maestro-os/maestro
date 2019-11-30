#include <acpi/aml/aml_parser.h>

static aml_node_t *source_buff(aml_parse_context_t *context)
{
	return parse_node(AML_SOURCE_BUFF, context, 1, term_arg);
}

static aml_node_t *bit_index(aml_parse_context_t *context)
{
	return parse_node(AML_BIT_INDEX, context, 1, term_arg);
}

static aml_node_t *byte_index(aml_parse_context_t *context)
{
	return parse_node(AML_BYTE_INDEX, context, 1, term_arg);
}

static aml_node_t *num_bits(aml_parse_context_t *context)
{
	return parse_node(AML_NUM_BITS, context, 1, term_arg);
}

static aml_node_t *def_create_bit_field(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *n;

	BLOB_COPY(context, &c);
	if(!BLOB_CHECK(context, CREATE_BIT_FIELD_OP))
		return NULL;
	if(!(n = parse_node(AML_DEF_CREATE_BIT_FIELD, context, 3,
		source_buff, bit_index, name_string)))
		BLOB_COPY(&c, context);
	return n;
}

static aml_node_t *def_create_byte_field(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *n;

	BLOB_COPY(context, &c);
	if(!BLOB_CHECK(context, CREATE_BYTE_FIELD_OP))
		return NULL;
	if(!(n = parse_node(AML_DEF_CREATE_BYTE_FIELD, context, 3,
		source_buff, byte_index, name_string)))
		BLOB_COPY(&c, context);
	return n;
}

static aml_node_t *def_create_dword_field(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *n;

	BLOB_COPY(context, &c);
	if(!BLOB_CHECK(context, CREATE_DWORD_FIELD_OP))
		return NULL;
	if(!(n = parse_node(AML_DEF_CREATE_DWORD_FIELD, context, 3,
		source_buff, byte_index, name_string)))
		BLOB_COPY(&c, context);
	return n;
}

static aml_node_t *def_create_field(aml_parse_context_t *context)
{
	// TODO
	(void) context;
	(void) num_bits;
	return NULL;
}

static aml_node_t *def_create_qword_field(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *n;

	BLOB_COPY(context, &c);
	if(!BLOB_CHECK(context, CREATE_QWORD_FIELD_OP))
		return NULL;
	if(!(n = parse_node(AML_DEF_CREATE_QWORD_FIELD, context, 3,
		source_buff, byte_index, name_string)))
		BLOB_COPY(&c, context);
	return n;
}

static aml_node_t *def_create_word_field(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *n;

	BLOB_COPY(context, &c);
	if(!BLOB_CHECK(context, CREATE_WORD_FIELD_OP))
		return NULL;
	if(!(n = parse_node(AML_DEF_CREATE_WORD_FIELD, context, 3,
		source_buff, byte_index, name_string)))
		BLOB_COPY(&c, context);
	return n;
}

static aml_node_t *def_data_region(aml_parse_context_t *context)
{
	// TODO
	(void) context;
	return NULL;
}

static aml_node_t *def_device(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *node;

	BLOB_COPY(context, &c);
	if(!BLOB_CHECK(context, EXT_OP_PREFIX) || !BLOB_CHECK(context, DEVICE_OP))
	{
		BLOB_COPY(&c, context);
		return NULL;
	}
	if(!(node = parse_explicit(AML_DEF_DEVICE, context,
		3, pkg_length, name_string, term_list)))
		BLOB_COPY(&c, context);
	return node;
}

static aml_node_t *def_external(aml_parse_context_t *context)
{
	// TODO
	(void) context;
	return NULL;
}

static aml_node_t *def_field(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *node;

	BLOB_COPY(context, &c);
	if(!BLOB_CHECK(context, EXT_OP_PREFIX) || !BLOB_CHECK(context, FIELD_OP))
	{
		BLOB_COPY(&c, context);
		return NULL;
	}
	if(!(node = parse_explicit(AML_DEF_FIELD, context,
		4, pkg_length, name_string, field_flags, field_list)))
		BLOB_COPY(&c, context);
	return node;
}

static aml_node_t *method_flags(aml_parse_context_t *context)
{
	return parse_node(AML_METHOD_FLAGS, context, 1, byte_data);
}

static aml_node_t *register_method(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *node;
	size_t len;

	printf("register_method\n");
	BLOB_COPY(context, &c);
	if(!(node = parse_node(AML_DEF_METHOD, context,
		2, pkg_length, name_string)))
		return NULL;
	len = aml_pkg_length_get(node->children);
	aml_method_insert(&context->methods, node);
	BLOB_CONSUME(&c, len);
	BLOB_COPY(&c, context);
	return node;
}

static aml_node_t *def_method(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *n;

	BLOB_COPY(context, &c);
	if(!BLOB_CHECK(context, METHOD_OP))
		return NULL;
	if(context->decl)
	{
		if(!(n = register_method(context)))
			BLOB_COPY(&c, context);
		return n;
	}
	if(!(n = parse_explicit(AML_DEF_METHOD, context,
		4, pkg_length, name_string, method_flags, term_list)))
		BLOB_COPY(&c, context);
	return n;
}

static aml_node_t *sync_flags(aml_parse_context_t *context)
{
	return parse_node(AML_SYNC_FLAGS, context, 1, byte_data);
}

static aml_node_t *def_mutex(aml_parse_context_t *context)
{
	return parse_operation(1, MUTEX_OP, AML_DEF_MUTEX, context,
		2, name_string, sync_flags);
}

static aml_node_t *def_power_res(aml_parse_context_t *context)
{
	// TODO
	(void) context;
	return NULL;
}

static aml_node_t *def_processor(aml_parse_context_t *context)
{
	// TODO
	(void) context;
	return NULL;
}

static aml_node_t *def_thermal_zone(aml_parse_context_t *context)
{
	// TODO
	(void) context;
	return NULL;
}

// TODO Cleanup
aml_node_t *named_obj(aml_parse_context_t *context)
{
	return parse_either(AML_NAMED_OBJ, context,
		14, def_bank_field, def_create_bit_field, def_create_byte_field,
			def_create_dword_field, def_create_field, def_create_qword_field,
				def_create_word_field, def_data_region, def_device,
					def_external, def_field, def_method, def_mutex,
						def_op_region, def_power_res, def_processor,
							def_thermal_zone);
}
