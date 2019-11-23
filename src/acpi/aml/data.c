#include <acpi/aml/aml_parser.h>

static aml_node_t *byte_const(blob_t *blob)
{
	blob_t b;
	aml_node_t *node;

	BLOB_COPY(blob, &b);
	if(BLOB_REMAIN(blob) < 2 || !BLOB_CHECK(blob, BYTE_PREFIX))
		return NULL;
	if(!(node = parse_node(AML_BYTE_CONST, blob, 1, byte_data)))
		BLOB_COPY(&b, blob);
	return node;
}

static aml_node_t *word_const(blob_t *blob)
{
	blob_t b;
	aml_node_t *node;

	if(BLOB_REMAIN(blob) < 3 || !BLOB_CHECK(blob, WORD_PREFIX))
		return NULL;
	BLOB_COPY(blob, &b);
	BLOB_CONSUME(blob, 1);
	if(!(node = parse_node(AML_WORD_CONST, blob, 1, word_data)))
		BLOB_COPY(&b, blob);
	return node;
}

static aml_node_t *dword_const(blob_t *blob)
{
	blob_t b;
	aml_node_t *node;

	if(BLOB_REMAIN(blob) < 5 || !BLOB_CHECK(blob, DWORD_PREFIX))
		return NULL;
	BLOB_COPY(blob, &b);
	BLOB_CONSUME(blob, 1);
	if(!(node = parse_node(AML_DWORD_CONST, blob, 1, dword_data)))
		BLOB_COPY(&b, blob);
	return node;
}

static aml_node_t *qword_const(blob_t *blob)
{
	blob_t b;
	aml_node_t *node;

	if(BLOB_REMAIN(blob) < 9 || !BLOB_CHECK(blob, QWORD_PREFIX))
		return NULL;
	BLOB_COPY(blob, &b);
	BLOB_CONSUME(blob, 1);
	if(!(node = parse_node(AML_QWORD_CONST, blob, 1, qword_data)))
		BLOB_COPY(&b, blob);
	return node;
}

static aml_node_t *const_obj(blob_t *blob)
{
	blob_t b;
	aml_node_t *node;

	if(BLOB_EMPTY(blob) || (BLOB_PEEK(blob) != ZERO_OP
		&& BLOB_PEEK(blob) != ONE_OP && BLOB_PEEK(blob) != ONES_OP))
		return NULL;
	BLOB_COPY(blob, &b);
	BLOB_CONSUME(blob, 1);
	if(!(node = node_new(AML_CONST_OBJ, &BLOB_PEEK(blob), 1)))
		BLOB_COPY(&b, blob);
	return node;
}

aml_node_t *byte_list(blob_t *blob, const size_t n)
{
	return parse_fixed_list(AML_BYTE_LIST, blob, byte_data, n);
}

static aml_node_t *revision_op(blob_t *blob)
{
	blob_t b;
	aml_node_t *node;

	if(!BLOB_CHECK(blob, EXT_OP_PREFIX) || !BLOB_CHECK(blob, REVISION_OP))
		return NULL;
	if((node = node_new(AML_REVISION_OP, &BLOB_PEEK(blob), 2)))
		BLOB_CONSUME(&b, 2);
	return node;
}

static aml_node_t *computational_data(blob_t *blob)
{
	return parse_either(AML_COMPUTATIONAL_DATA, blob,
		8, byte_const, word_const, dword_const, qword_const,
			string, const_obj, revision_op, def_buffer);
}

aml_node_t *data_object(blob_t *blob)
{
	return parse_either(AML_DATA_OBJECT, blob,
		3, computational_data, def_package, def_var_package);
}

aml_node_t *byte_data(blob_t *blob)
{
	aml_node_t *node;

	if(BLOB_EMPTY(blob)
		|| !(node = node_new(AML_BYTE_DATA, &BLOB_PEEK(blob), 1)))
		return NULL;
	BLOB_CONSUME(blob, 1);
	return node;
}

aml_node_t *word_data(blob_t *blob)
{
	return parse_node(AML_WORD_DATA, blob, 2, byte_data, byte_data);
}

aml_node_t *dword_data(blob_t *blob)
{
	return parse_node(AML_DWORD_DATA, blob, 2, word_data, word_data);
}

aml_node_t *qword_data(blob_t *blob)
{
	return parse_node(AML_QWORD_DATA, blob, 2, dword_data, dword_data);
}
