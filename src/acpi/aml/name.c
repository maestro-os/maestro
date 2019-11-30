#include <acpi/aml/aml_parser.h>

static aml_node_t *root_char(aml_parse_context_t *context)
{
	aml_node_t *node;

	if(BLOB_EMPTY(context) || !IS_ROOT_CHAR(BLOB_PEEK(context)))
		return NULL;
	if(!(node = node_new(AML_ROOT_CHAR, &BLOB_PEEK(context), 1)))
		return NULL;
	BLOB_CONSUME(context, 1);
	return node;
}

static aml_node_t *prefix_path(aml_parse_context_t *context)
{
	aml_node_t *node;

	if(!(node = node_new(AML_PREFIX_PATH, &BLOB_PEEK(context), 1)))
		return NULL;
	if(!BLOB_EMPTY(context) && IS_PREFIX_CHAR(BLOB_PEEK(context)))
	{
		BLOB_CONSUME(context, 1);
		node_add_child(node, prefix_path(context)); // TODO Check errno
	}
	return node;
}

// TODO Is buffer needed?
aml_node_t *name_seg(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	char buff[4];
	size_t i = 0;
	aml_node_t *node;

	if(BLOB_EMPTY(context) || !IS_LEAD_NAME_CHAR(BLOB_PEEK(context)))
		return NULL;
	BLOB_COPY(context, &c);
	memset(buff, '_', sizeof(buff));
	while(i < sizeof(buff)
		&& !BLOB_EMPTY(context) && IS_NAME_CHAR(BLOB_PEEK(context)))
	{
		buff[i++] = BLOB_PEEK(context);
		BLOB_CONSUME(context, 1);
	}
	if(!(node = node_new(AML_NAME_SEG, buff, sizeof(buff))))
		BLOB_COPY(&c, context);
	return node;
}

static aml_node_t *dual_name_path(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *node;

	BLOB_COPY(context, &c);
	if(!BLOB_CHECK(context, DUAL_NAME_PREFIX))
		return NULL;
	if(!(node = parse_node(AML_DUAL_NAME_PATH, context, 2, name_seg, name_seg)))
		BLOB_COPY(&c, context);
	return node;
}

// TODO Clean
static aml_node_t *multi_name_path(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *child, *node;
	size_t i = 0, n;

	BLOB_COPY(context, &c);
	if(BLOB_REMAIN(context) < 2 || !BLOB_CHECK(context, MULTI_NAME_PREFIX)
		|| !(node = node_new(AML_MULTI_NAME_PATH, &BLOB_PEEK(context), 0)))
		return NULL;
	n = BLOB_PEEK(context);
	BLOB_CONSUME(context, 1);
	while(i++ < n && (child = name_seg(context)))
		node_add_child(node, child);
	if(!child)
	{
		BLOB_COPY(&c, context);
		ast_free(node);
		return NULL;
	}
	return node;
}

aml_node_t *simple_name(aml_parse_context_t *context)
{
	return parse_either(AML_SIMPLE_NAME, context,
		3, name_string, arg_obj, local_obj);
}

aml_node_t *null_name(aml_parse_context_t *context)
{
	aml_node_t *node;

	if(BLOB_EMPTY(context) || BLOB_PEEK(context))
		return NULL;
	if((node = node_new(AML_NULL_NAME, &BLOB_PEEK(context), 1)))
		BLOB_CONSUME(context, 1);
	return node;
}

aml_node_t *super_name(aml_parse_context_t *context)
{
	return parse_either(AML_SUPER_NAME, context,
		3, simple_name, debug_obj, type6_opcode);
}

static aml_node_t *name_path(aml_parse_context_t *context)
{
	return parse_either(AML_NAME_PATH, context,
		4, name_seg, dual_name_path, multi_name_path, null_name);
}

aml_node_t *name_string(aml_parse_context_t *context)
{
	aml_parse_context_t c;
	aml_node_t *node, *n;

	if(BLOB_EMPTY(context)
		|| !(node = node_new(AML_NAME_STRING, &BLOB_PEEK(context), 0)))
		return NULL;
	BLOB_COPY(context, &c);
	if(!(n = root_char(context)) && !(n = prefix_path(context)))
		goto fail;
	node_add_child(node, n);
	if(!(n = name_path(context)))
		goto fail;
	node_add_child(node, n);
	return node;

fail:
	BLOB_COPY(&c, context);
	ast_free(node);
	return NULL;
}
