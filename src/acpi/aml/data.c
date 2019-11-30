#include <acpi/aml/aml_parser.h>

static aml_node_t *byte_const(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *node;

	BLOB_COPY(context, &c);
	if(BLOB_REMAIN(context) < 2 || !BLOB_CHECK(context, BYTE_PREFIX))
		return NULL;
	if(!(node = parse_node(AML_BYTE_CONST, context, 1, byte_data)))
		BLOB_COPY(&c, context);
	return node;
}

static aml_node_t *word_const(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *node;

	BLOB_COPY(context, &c);
	if(BLOB_REMAIN(context) < 3 || !BLOB_CHECK(context, WORD_PREFIX))
		return NULL;
	if(!(node = parse_node(AML_WORD_CONST, context, 1, word_data)))
		BLOB_COPY(&c, context);
	return node;
}

static aml_node_t *dword_const(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *node;

	BLOB_COPY(context, &c);
	if(BLOB_REMAIN(context) < 5 || !BLOB_CHECK(context, DWORD_PREFIX))
		return NULL;
	if(!(node = parse_node(AML_DWORD_CONST, context, 1, dword_data)))
		BLOB_COPY(&c, context);
	return node;
}

static aml_node_t *qword_const(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *node;

	BLOB_COPY(context, &c);
	if(BLOB_REMAIN(context) < 9 || !BLOB_CHECK(context, QWORD_PREFIX))
		return NULL;
	if(!(node = parse_node(AML_QWORD_CONST, context, 1, qword_data)))
		BLOB_COPY(&c, context);
	return node;
}

static aml_node_t *const_obj(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *node;

	if(BLOB_PEEK(context) != ZERO_OP && BLOB_PEEK(context) != ONE_OP
		&& BLOB_PEEK(context) != ONES_OP)
		return NULL;
	BLOB_COPY(context, &c);
	BLOB_CONSUME(context, 1);
	if(!(node = node_new(AML_CONST_OBJ, &BLOB_PEEK(context), 1)))
		BLOB_COPY(&c, context);
	return node;
}

aml_node_t *byte_list(aml_parse_context_t *context, const size_t n)
{
	return parse_fixed_list(AML_BYTE_LIST, context, byte_data, n);
}

static aml_node_t *revision_op(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *node;

	BLOB_COPY(context, &c);
	if(!BLOB_CHECK(context, EXT_OP_PREFIX)
		|| !BLOB_CHECK(context, REVISION_OP))
	{
		BLOB_COPY(&c, context);
		return NULL;
	}
	if(!(node = node_new(AML_REVISION_OP, &BLOB_PEEK(context), 2)))
		BLOB_COPY(&c, context);
	return node;
}

static aml_node_t *computational_data(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *node;

	BLOB_COPY(context, &c);
	if(!BLOB_EMPTY(context) && BLOB_PEEK(context) == BUFFER_OP) // TODO remove?
	{
		BLOB_CONSUME(context, 1);
		if(!(node = def_buffer(context)))
			BLOB_COPY(&c, context);
		return node;
	}
	return parse_either(AML_COMPUTATIONAL_DATA, context,
		7, byte_const, word_const, dword_const, qword_const, string, const_obj,
			revision_op);
}

aml_node_t *data_object(aml_parse_context_t *context)
{
	return parse_either(AML_DATA_OBJECT, context,
		3, computational_data, def_package, def_var_package);
}

aml_node_t *byte_data(aml_parse_context_t *context)
{
	aml_node_t *node;

	if(BLOB_EMPTY(context)
		|| !(node = node_new(AML_BYTE_DATA, &BLOB_PEEK(context), 1)))
		return NULL;
	BLOB_CONSUME(context, 1);
	return node;
}

aml_node_t *word_data(aml_parse_context_t *context)
{
	return parse_node(AML_WORD_DATA, context, 2, byte_data, byte_data);
}

aml_node_t *dword_data(aml_parse_context_t *context)
{
	return parse_node(AML_DWORD_DATA, context, 2, word_data, word_data);
}

aml_node_t *qword_data(aml_parse_context_t *context)
{
	return parse_node(AML_QWORD_DATA, context, 2, dword_data, dword_data);
}
