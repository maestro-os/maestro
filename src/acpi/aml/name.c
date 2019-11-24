#include <acpi/aml/aml_parser.h>

static aml_node_t *root_char(blob_t *blob)
{
	aml_node_t *node;

	if(BLOB_EMPTY(blob) || !IS_ROOT_CHAR(BLOB_PEEK(blob)))
		return NULL;
	if(!(node = node_new(AML_ROOT_CHAR, &BLOB_PEEK(blob), 1)))
		return NULL;
	BLOB_CONSUME(blob, 1);
	return node;
}

static aml_node_t *prefix_path(blob_t *blob)
{
	aml_node_t *node;

	if(!(node = node_new(AML_PREFIX_PATH, &BLOB_PEEK(blob), 1)))
		return NULL;
	if(!BLOB_EMPTY(blob) && IS_PREFIX_CHAR(BLOB_PEEK(blob)))
	{
		BLOB_CONSUME(blob, 1);
		node->children = prefix_path(blob); // TODO Check errno
	}
	return node;
}

// TODO Is buffer needed?
aml_node_t *name_seg(blob_t *blob)
{
	blob_t b;
	char buff[4];
	size_t i = 0;
	aml_node_t *node;

	if(BLOB_EMPTY(blob) || !IS_LEAD_NAME_CHAR(BLOB_PEEK(blob)))
		return NULL;
	BLOB_COPY(blob, &b);
	memset(buff, '_', sizeof(buff));
	while(i < sizeof(buff)
		&& !BLOB_EMPTY(blob) && IS_NAME_CHAR(BLOB_PEEK(blob)))
	{
		buff[i++] = BLOB_PEEK(blob);
		BLOB_CONSUME(blob, 1);
	}
	if(!(node = node_new(AML_NAME_SEG, buff, sizeof(buff))))
		BLOB_COPY(&b, blob);
	return node;
}

static aml_node_t *dual_name_path(blob_t *blob)
{
	blob_t b;
	aml_node_t *node;

	BLOB_COPY(blob, &b);
	if(!BLOB_CHECK(blob, DUAL_NAME_PREFIX))
		return NULL;
	if(!(node = parse_node(AML_DUAL_NAME_PATH, blob, 2, name_seg, name_seg)))
		BLOB_COPY(&b, blob);
	return node;
}

// TODO Clean
static aml_node_t *multi_name_path(blob_t *blob)
{
	blob_t b;
	aml_node_t *c, *node;
	size_t i = 0, n;

	BLOB_COPY(blob, &b);
	if(BLOB_REMAIN(blob) < 2 || !BLOB_CHECK(blob, MULTI_NAME_PREFIX)
		|| !(node = node_new(AML_MULTI_NAME_PATH, &BLOB_PEEK(blob), 0)))
		return NULL;
	n = BLOB_PEEK(blob);
	BLOB_CONSUME(blob, 1);
	while(i++ < n && (c = name_seg(blob)))
		node_add_child(node, c);
	if(!c)
	{
		BLOB_COPY(&b, blob);
		ast_free(node);
		return NULL;
	}
	return node;
}

aml_node_t *simple_name(blob_t *blob)
{
	return parse_either(AML_SIMPLE_NAME, blob,
		3, name_string, arg_obj, local_obj);
}

aml_node_t *null_name(blob_t *blob)
{
	aml_node_t *node;

	if(BLOB_EMPTY(blob) || BLOB_PEEK(blob))
		return NULL;
	if((node = node_new(AML_NULL_NAME, &BLOB_PEEK(blob), 1)))
		BLOB_CONSUME(blob, 1);
	return node;
}

aml_node_t *super_name(blob_t *blob)
{
	return parse_either(AML_SUPER_NAME, blob,
		3, simple_name, debug_obj, type6_opcode);
}

static aml_node_t *name_path(blob_t *blob)
{
	return parse_either(AML_NAME_PATH, blob,
		4, name_seg, dual_name_path, multi_name_path, null_name);
}

aml_node_t *name_string(blob_t *blob)
{
	blob_t b;
	aml_node_t *node;

	if(BLOB_EMPTY(blob)
		|| !(node = node_new(AML_NAME_STRING, &BLOB_PEEK(blob), 0)))
		return NULL;
	BLOB_COPY(blob, &b);
	if(!(node->children = root_char(blob))
		&& !(node->children = prefix_path(blob)))
		goto fail;
	if(!(node->children->next = name_path(blob)))
		goto fail;
	return node;

fail:
	BLOB_COPY(&b, blob);
	ast_free(node);
	return NULL;
}
