#include <acpi/aml/aml_parser.h>
#include <stdarg.h>

#ifdef KERNEL_DEBUG
# include <tty/tty.h> // TODO remove

static const char *node_types[] = {
	[AML_CODE] = "AML_CODE",
	[AML_DEF_BLOCK_HEADER] = "DEF_BLOCK_HEADER",
	[AML_TABLE_SIGNATURE] = "TABLE_SIGNATURE",
	[AML_TABLE_LENGTH] = "TABLE_LENGTH",
	[AML_SPEC_COMPLIANCE] = "SPEC_COMPLIANCE",
	[AML_CHECK_SUM] = "CHECK_SUM",
	[AML_OEM_ID] = "OEM_ID",
	[AML_OEM_TABLE_ID] = "OEM_TABLE_ID",
	[AML_OEM_REVISION] = "OEM_REVISION",
	[AML_CREATOR_ID] = "CREATOR_ID",
	[AML_CREATOR_REVISION] = "CREATOR_REVISION",
	[AML_ROOT_CHAR] = "ROOT_CHAR",
	[AML_NAME_SEG] = "NAME_SEG",
	[AML_NAME_STRING] = "NAME_STRING",
	[AML_PREFIX_PATH] = "PREFIX_PATH",
	[AML_NAME_PATH] = "NAME_PATH",
	[AML_DUAL_NAME_PATH] = "DUAL_NAME_PATH",
	[AML_MULTI_NAME_PATH] = "MULTI_NAME_PATH",
	[AML_SEG_COUNT] = "SEG_COUNT",
	[AML_SIMPLE_NAME] = "SIMPLE_NAME",
	[AML_SUPER_NAME] = "SUPER_NAME",
	[AML_NULL_NAME] = "NULL_NAME",
	[AML_TARGET] = "TARGET",
	[AML_COMPUTATIONAL_DATA] = "COMPUTATIONAL_DATA",
	[AML_DATA_OBJECT] = "DATA_OBJECT",
	[AML_DATA_REF_OBJECT] = "DATA_REF_OBJECT",
	[AML_BYTE_CONST] = "BYTE_CONST",
	[AML_BYTE_PREFIX] = "BYTE_PREFIX",
	[AML_WORD_CONST] = "WORD_CONST",
	[AML_WORD_PREFIX] = "WORD_PREFIX",
	[AML_DWORD_CONST] = "D_WORD_CONST",
	[AML_DWORD_PREFIX] = "D_WORD_PREFIX",
	[AML_QWORD_CONST] = "Q_WORD_CONST",
	[AML_QWORD_PREFIX] = "Q_WORD_PREFIX",
	[AML_STRING] = "STRING",
	[AML_STRING_PREFIX] = "STRING_PREFIX",
	[AML_CONST_OBJ] = "CONST_OBJ",
	[AML_BYTE_LIST] = "BYTE_LIST",
	[AML_BYTE_DATA] = "BYTE_DATA",
	[AML_WORD_DATA] = "WORD_DATA",
	[AML_DWORD_DATA] = "DWORD_DATA",
	[AML_QWORD_DATA] = "QWORD_DATA",
	[AML_ASCII_CHAR_LIST] = "ASCII_CHAR_LIST",
	[AML_ASCII_CHAR] = "ASCII_CHAR",
	[AML_NULL_CHAR] = "NULL_CHAR",
	[AML_ZERO_OP] = "ZERO_OP",
	[AML_ONE_OP] = "ONE_OP",
	[AML_ONES_OP] = "ONES_OP",
	[AML_REVISION_OP] = "REVISION_OP",
	[AML_PKG_LENGTH] = "PKG_LENGTH",
	[AML_PKG_LEAD_BYTE] = "PKG_LEAD_BYTE",
	[AML_OBJECT] = "OBJECT",
	[AML_TERM_OBJ] = "TERM_OBJ",
	[AML_TERM_LIST] = "TERM_LIST",
	[AML_TERM_ARG] = "TERM_ARG",
	[AML_METHOD_INVOCATION] = "METHOD_INVOCATION",
	[AML_TERM_ARG_LIST] = "TERM_ARG_LIST",
	[AML_NAME_SPACE_MODIFIER_OBJ] = "NAME_SPACE_MODIFIER_OBJ",
	[AML_DEF_ALIAS] = "DEF_ALIAS",
	[AML_DEF_NAME] = "DEF_NAME",
	[AML_DEF_SCOPE] = "DEF_SCOPE",
	[AML_NAMED_OBJ] = "NAMED_OBJ",
	[AML_DEF_BANK_FIELD] = "DEF_BANK_FIELD",
	[AML_BANK_VALUE] = "BANK_VALUE",
	[AML_FIELD_FLAGS] = "FIELD_FLAGS",
	[AML_FIELD_LIST] = "FIELD_LIST",
	[AML_NAMED_FIELD] = "NAMED_FIELD",
	[AML_RESERVED_FIELD] = "RESERVED_FIELD",
	[AML_ACCESS_FIELD] = "ACCESS_FIELD",
	[AML_ACCESS_TYPE] = "ACCESS_TYPE",
	[AML_ACCESS_ATTRIB] = "ACCESS_ATTRIB",
	[AML_CONNECT_FIELD] = "CONNECT_FIELD",
	[AML_DEF_CREATE_BIT_FIELD] = "DEF_CREATE_BIT_FIELD",
	[AML_SOURCE_BUFF] = "SOURCE_BUFF",
	[AML_BIT_INDEX] = "BIT_INDEX",
	[AML_DEF_CREATE_BYTE_FIELD] = "DEF_CREATE_BYTE_FIELD",
	[AML_BYTE_INDEX] = "BYTE_INDEX",
	[AML_DEF_CREATE_DWORD_FIELD] = "DEF_CREATE_D_WORD_FIELD",
	[AML_DEF_CREATE_FIELD] = "DEF_CREATE_FIELD",
	[AML_NUM_BITS] = "NUM_BITS",
	[AML_DEF_CREATE_QWORD_FIELD] = "DEF_CREATE_Q_WORD_FIELD",
	[AML_DEF_CREATE_WORD_FIELD] = "DEF_CREATE_WORD_FIELD",
	[AML_DEF_DATA_REGION] = "DEF_DATA_REGION",
	[AML_DATA_REGION_OP] = "DATA_REGION_OP",
	[AML_DEF_DEVICE] = "DEF_DEVICE",
	[AML_DEVICE_OP] = "DEVICE_OP",
	[AML_DEF_EVENT] = "DEF_EVENT",
	[AML_EVENT_OP] = "EVENT_OP",
	[AML_DEF_EXTERNAL] = "DEF_EXTERNAL",
	[AML_EXTERNAL_OP] = "EXTERNAL_OP",
	[AML_OBJECT_TYPE] = "OBJECT_TYPE",
	[AML_ARGUMENT_COUNT] = "ARGUMENT_COUNT",
	[AML_DEF_FIELD] = "DEF_FIELD",
	[AML_FIELD_OP] = "FIELD_OP",
	[AML_DEF_INDEX_FIELD] = "DEF_INDEX_FIELD",
	[AML_INDEX_FIELD_OP] = "INDEX_FIELD_OP",
	[AML_DEF_METHOD] = "DEF_METHOD",
	[AML_METHOD_OP] = "METHOD_OP",
	[AML_METHOD_FLAGS] = "METHOD_FLAGS",
	[AML_DEF_MUTEX] = "DEF_MUTEX",
	[AML_MUTEX_OP] = "MUTEX_OP",
	[AML_SYNC_FLAGS] = "SYNC_FLAGS",
	[AML_DEF_OP_REGION] = "DEF_OP_REGION",
	[AML_OP_REGION_OP] = "OP_REGION_OP",
	[AML_REGION_SPACE] = "REGION_SPACE",
	[AML_REGION_OFFSET] = "REGION_OFFSET",
	[AML_REGION_LEN] = "REGION_LEN",
	[AML_DEF_POWER_RES] = "DEF_POWER_RES",
	[AML_POWER_RES_OP] = "POWER_RES_OP",
	[AML_SYSTEM_LEVEL] = "SYSTEM_LEVEL",
	[AML_RESOURCE_ORDER] = "RESOURCE_ORDER",
	[AML_DEF_PROCESSOR] = "DEF_PROCESSOR",
	[AML_PROCESSOR_OP] = "PROCESSOR_OP",
	[AML_PROC_ID] = "PROC_ID",
	[AML_PBLK_ADDR] = "PBLK_ADDR",
	[AML_PBLK_LEN] = "PBLK_LEN",
	[AML_DEF_THERMAL_ZONE] = "DEF_THERMAL_ZONE",
	[AML_THERMAL_ZONE_OP] = "THERMAL_ZONE_OP",
	[AML_EXTENDED_ACCESS_FIELD] = "EXTENDED_ACCESS_FIELD",
	[AML_EXTENDED_ACCESS_ATTRIB] = "EXTENDED_ACCESS_ATTRIB",
	[AML_FIELD_ELEMENT] = "FIELD_ELEMENT",
	[AML_TYPE1_OPCODE] = "TYPE1_OPCODE",
	[AML_DEF_BREAK] = "DEF_BREAK",
	[AML_DEF_BREAK_POINT] = "DEF_BREAK_POINT",
	[AML_DEF_CONTINUE] = "DEF_CONTINUE",
	[AML_DEF_ELSE] = "DEF_ELSE",
	[AML_DEF_FATAL] = "DEF_FATAL",
	[AML_FATAL_OP] = "FATAL_OP",
	[AML_FATAL_TYPE] = "FATAL_TYPE",
	[AML_FATAL_CODE] = "FATAL_CODE",
	[AML_FATAL_ARG] = "FATAL_ARG",
	[AML_DEF_IF_ELSE] = "DEF_IF_ELSE",
	[AML_PREDICATE] = "PREDICATE",
	[AML_DEF_LOAD] = "DEF_LOAD",
	[AML_LOAD_OP] = "LOAD_OP",
	[AML_DDB_HANDLE_OBJECT] = "DDB_HANDLE_OBJECT",
	[AML_DEF_NOOP] = "DEF_NOOP",
	[AML_DEF_NOTIFY] = "DEF_NOTIFY",
	[AML_NOTIFY_OP] = "NOTIFY_OP",
	[AML_NOTIFY_OBJECT] = "NOTIFY_OBJECT",
	[AML_NOTIFY_VALUE] = "NOTIFY_VALUE",
	[AML_DEF_RELEASE] = "DEF_RELEASE",
	[AML_RELEASE_OP] = "RELEASE_OP",
	[AML_MUTEX_OBJECT] = "MUTEX_OBJECT",
	[AML_DEF_RESET] = "DEF_RESET",
	[AML_RESET_OP] = "RESET_OP",
	[AML_EVENT_OBJECT] = "EVENT_OBJECT",
	[AML_DEF_RETURN] = "DEF_RETURN",
	[AML_RETURN_OP] = "RETURN_OP",
	[AML_ARG_OBJECT] = "ARG_OBJECT",
	[AML_DEF_SIGNAL] = "DEF_SIGNAL",
	[AML_SIGNAL_OP] = "SIGNAL_OP",
	[AML_DEF_SLEEP] = "DEF_SLEEP",
	[AML_SLEEP_OP] = "SLEEP_OP",
	[AML_MSEC_TIME] = "MSEC_TIME",
	[AML_DEF_STALL] = "DEF_STALL",
	[AML_STALL_OP] = "STALL_OP",
	[AML_USEC_TIME] = "USEC_TIME",
	[AML_DEF_WHILE] = "DEF_WHILE",
	[AML_WHILE_OP] = "WHILE_OP",
	[AML_TYPE2_OPCODE] = "TYPE2_OPCODE",
	[AML_TYPE6_OPCODE] = "TYPE6_OPCODE",
	[AML_DEF_ACQUIRE] = "DEF_ACQUIRE",
	[AML_ACQUIRE_OP] = "ACQUIRE_OP",
	[AML_TIMEOUT] = "TIMEOUT",
	[AML_DEF_ADD] = "DEF_ADD",
	[AML_ADD_OP] = "ADD_OP",
	[AML_OPERAND] = "OPERAND",
	[AML_DEF_AND] = "DEF_AND",
	[AML_AND_OP] = "AND_OP",
	[AML_DEF_BUFFER] = "DEF_BUFFER",
	[AML_BUFFER_OP] = "BUFFER_OP",
	[AML_BUFFER_SIZE] = "BUFFER_SIZE",
	[AML_DEF_CONCAT] = "DEF_CONCAT",
	[AML_CONCAT_OP] = "CONCAT_OP",
	[AML_DATA] = "DATA",
	[AML_DEF_CONCAT_RES] = "DEF_CONCAT_RES",
	[AML_CONCAT_RES_OP] = "CONCAT_RES_OP",
	[AML_BUF_DATA] = "BUF_DATA",
	[AML_DEF_COND_REF_OF] = "DEF_COND_REF_OF",
	[AML_COND_REF_OF_OP] = "COND_REF_OF_OP",
	[AML_DEF_COPY_OBJECT] = "DEF_COPY_OBJECT",
	[AML_COPY_OBJECT_OP] = "COPY_OBJECT_OP",
	[AML_DEF_DECREMENT] = "DEF_DECREMENT",
	[AML_DECREMENT_OP] = "DECREMENT_OP",
	[AML_DEF_DEREF_OF] = "DEF_DEREF_OF",
	[AML_DEREF_OF_OP] = "DEREF_OF_OP",
	[AML_OBJ_REFERENCE] = "OBJ_REFERENCE",
	[AML_DEF_DIVIDE] = "DEF_DIVIDE",
	[AML_DIVIDE_OP] = "DIVIDE_OP",
	[AML_DIVIDEND] = "DIVIDEND",
	[AML_DIVISOR] = "DIVISOR",
	[AML_REMAINDER] = "REMAINDER",
	[AML_QUOTIENT] = "QUOTIENT",
	[AML_DEF_FIND_SET_LEFT_BIT] = "DEF_FIND_SET_LEFT_BIT",
	[AML_FIND_SET_LEFT_BIT_OP] = "FIND_SET_LEFT_BIT_OP",
	[AML_DEF_FIND_SET_RIGHT_BIT] = "DEF_FIND_SET_RIGHT_BIT",
	[AML_FIND_SET_RIGHT_BIT_OP] = "FIND_SET_RIGHT_BIT_OP",
	[AML_DEF_FROM_BCD] = "DEF_FROM_BCD",
	[AML_FROM_BCD_OP] = "FROM_BCD_OP",
	[AML_BCD_VALUE] = "BCD_VALUE",
	[AML_DEF_INCREMENT] = "DEF_INCREMENT",
	[AML_INCREMENT_OP] = "INCREMENT_OP",
	[AML_DEF_INDEX] = "DEF_INDEX",
	[AML_INDEX_OP] = "INDEX_OP",
	[AML_BUFF_PKG_STR_OBJ] = "BUFF_PKG_STR_OBJ",
	[AML_INDEX_VALUE] = "INDEX_VALUE",
	[AML_DEF_L_AND] = "DEF_L_AND",
	[AML_LAND_OP] = "LAND_OP",
	[AML_DEF_L_EQUAL] = "DEF_L_EQUAL",
	[AML_LEQUAL_OP] = "LEQUAL_OP",
	[AML_DEF_L_GREATER] = "DEF_L_GREATER",
	[AML_LGREATER_OP] = "LGREATER_OP",
	[AML_DEF_L_GREATER_EQUAL] = "DEF_L_GREATER_EQUAL",
	[AML_LGREATER_EQUAL_OP] = "LGREATER_EQUAL_OP",
	[AML_DEF_L_LESS] = "DEF_L_LESS",
	[AML_LLESS_OP] = "LLESS_OP",
	[AML_DEF_L_LESS_EQUAL] = "DEF_L_LESS_EQUAL",
	[AML_LLESS_EQUAL_OP] = "LLESS_EQUAL_OP",
	[AML_DEF_L_NOT] = "DEF_L_NOT",
	[AML_LNOT_OP] = "LNOT_OP",
	[AML_DEF_L_NOT_EQUAL] = "DEF_L_NOT_EQUAL",
	[AML_LNOT_EQUAL_OP] = "LNOT_EQUAL_OP",
	[AML_DEF_LOAD_TABLE] = "DEF_LOAD_TABLE",
	[AML_LOAD_TABLE_OP] = "LOAD_TABLE_OP",
	[AML_DEF_L_OR] = "DEF_L_OR",
	[AML_LOR_OP] = "LOR_OP",
	[AML_DEF_MATCH] = "DEF_MATCH",
	[AML_MATCH_OP] = "MATCH_OP",
	[AML_SEARCH_PKG] = "SEARCH_PKG",
	[AML_MATCH_OPCODE] = "MATCH_OPCODE",
	[AML_START_INDEX] = "START_INDEX",
	[AML_DEF_MID] = "DEF_MID",
	[AML_MID_OP] = "MID_OP",
	[AML_MID_OBJ] = "MID_OBJ",
	[AML_DEF_MOD] = "DEF_MOD",
	[AML_MOD_OP] = "MOD_OP",
	[AML_DEF_MULTIPLY] = "DEF_MULTIPLY",
	[AML_MULTIPLY_OP] = "MULTIPLY_OP",
	[AML_DEF_N_AND] = "DEF_N_AND",
	[AML_NAND_OP] = "NAND_OP",
	[AML_DEF_N_OR] = "DEF_N_OR",
	[AML_NOR_OP] = "NOR_OP",
	[AML_DEF_NOT] = "DEF_NOT",
	[AML_NOT_OP] = "NOT_OP",
	[AML_DEF_OBJECT_TYPE] = "DEF_OBJECT_TYPE",
	[AML_OBJECT_TYPE_OP] = "OBJECT_TYPE_OP",
	[AML_DEF_OR] = "DEF_OR",
	[AML_OR_OP] = "OR_OP",
	[AML_DEF_PACKAGE] = "DEF_PACKAGE",
	[AML_PACKAGE_OP] = "PACKAGE_OP",
	[AML_DEF_VAR_PACKAGE] = "DEF_VAR_PACKAGE",
	[AML_VAR_PACKAGE_OP] = "VAR_PACKAGE_OP",
	[AML_NUM_ELEMENTS] = "NUM_ELEMENTS",
	[AML_VAR_NUM_ELEMENTS] = "VAR_NUM_ELEMENTS",
	[AML_PACKAGE_ELEMENT_LIST] = "PACKAGE_ELEMENT_LIST",
	[AML_PACKAGE_ELEMENT] = "PACKAGE_ELEMENT",
	[AML_DEF_REF_OF] = "DEF_REF_OF",
	[AML_REF_OF_OP] = "REF_OF_OP",
	[AML_DEF_SHIFT_LEFT] = "DEF_SHIFT_LEFT",
	[AML_SHIFT_LEFT_OP] = "SHIFT_LEFT_OP",
	[AML_SHIFT_COUNT] = "SHIFT_COUNT",
	[AML_DEF_SHIFT_RIGHT] = "DEF_SHIFT_RIGHT",
	[AML_SHIFT_RIGHT_OP] = "SHIFT_RIGHT_OP",
	[AML_DEF_SIZE_OF] = "DEF_SIZE_OF",
	[AML_SIZE_OF_OP] = "SIZE_OF_OP",
	[AML_DEF_STORE] = "DEF_STORE",
	[AML_STORE_OP] = "STORE_OP",
	[AML_DEF_SUBTRACT] = "DEF_SUBTRACT",
	[AML_SUBTRACT_OP] = "SUBTRACT_OP",
	[AML_DEF_TIMER] = "DEF_TIMER",
	[AML_TIMER_OP] = "TIMER_OP",
	[AML_DEF_TO_BCD] = "DEF_TO_BCD",
	[AML_TO_BCD_OP] = "TO_BCD_OP",
	[AML_DEF_TO_BUFFER] = "DEF_TO_BUFFER",
	[AML_TO_BUFFER_OP] = "TO_BUFFER_OP",
	[AML_DEF_TO_DECIMAL_STRING] = "DEF_TO_DECIMAL_STRING",
	[AML_TO_DECIMAL_STRING_OP] = "TO_DECIMAL_STRING_OP",
	[AML_DEF_TO_HEX_STRING] = "DEF_TO_HEX_STRING",
	[AML_TO_HEX_STRING_OP] = "TO_HEX_STRING_OP",
	[AML_DEF_TO_INTEGER] = "DEF_TO_INTEGER",
	[AML_TO_INTEGER_OP] = "TO_INTEGER_OP",
	[AML_DEF_TO_STRING] = "DEF_TO_STRING",
	[AML_LENGTH_ARG] = "LENGTH_ARG",
	[AML_TO_STRING_OP] = "TO_STRING_OP",
	[AML_DEF_WAIT] = "DEF_WAIT",
	[AML_WAIT_OP] = "WAIT_OP",
	[AML_DEF_XOR] = "DEF_X_OR",
	[AML_XOR_OP] = "XOR_OP",
	[AML_ARG_OBJ] = "ARG_OBJ",
	[AML_ARG0_OP] = "ARG0_OP",
	[AML_ARG1_OP] = "ARG1_OP",
	[AML_ARG2_OP] = "ARG2_OP",
	[AML_ARG3_OP] = "ARG3_OP",
	[AML_ARG4_OP] = "ARG4_OP",
	[AML_ARG5_OP] = "ARG5_OP",
	[AML_ARG6_OP] = "ARG6_OP",
	[AML_LOCAL_OBJ] = "LOCAL_OBJ",
	[AML_LOCAL0_OP] = "LOCAL0_OP",
	[AML_LOCAL1_OP] = "LOCAL1_OP",
	[AML_LOCAL2_OP] = "LOCAL2_OP",
	[AML_LOCAL3_OP] = "LOCAL3_OP",
	[AML_LOCAL4_OP] = "LOCAL4_OP",
	[AML_LOCAL5_OP] = "LOCAL5_OP",
	[AML_LOCAL6_OP] = "LOCAL6_OP",
	[AML_LOCAL7_OP] = "LOCAL7_OP",
	[AML_DEBUG_OBJ] = "DEBUG_OBJ",
	[AML_DEBUG_OP] = "DEBUG_OP"
};
#endif

static aml_node_t *do_parse(aml_parse_context_t *context, size_t n, va_list ap)
{
	aml_parse_context_t c;
	aml_node_t *node, *children = NULL, *last_child = NULL;

	BLOB_COPY(context, &c);
	while(n-- > 0)
	{
		node = va_arg(ap, parse_func_t)(context);
		if(!node)
			goto fail;
		if(!last_child)
			last_child = children = node;
		else
		{
			last_child->next = node;
			last_child = node;
		}
	}
	va_end(ap);
	return children;

fail:
	BLOB_COPY(&c, context);
	ast_free(children);
	return NULL;
}

aml_node_t *parse_node(const enum node_type type, aml_parse_context_t *context,
	const size_t n, ...)
{
	va_list ap;
	aml_node_t *children, *node = NULL;

	va_start(ap, n);
	if(!(node = node_new(type, &BLOB_PEEK(context), 0))
		|| !(children = do_parse(context, n, ap)))
	{
		node_free(node);
		return NULL;
	}
	node_add_child(node, children);
	return node;
}

aml_node_t *parse_explicit(const enum node_type type,
	aml_parse_context_t *context, size_t n, ...)
{
	va_list ap;
	aml_parse_context_t c, context2;
	aml_node_t *nod = NULL, *node = NULL, *children;
	size_t total_len, len;

	va_start(ap, n);
	BLOB_COPY(context, &c);
	if(!(nod = va_arg(ap, parse_func_t)(context)))
		return NULL;
	total_len = aml_pkg_length_get(nod);
	len = total_len - (c.len - context->len);
	printf("pkg_length is %u bytes long\n", (unsigned) (c.len - context->len));
	if(len > c.len)
	{
		printf("doesn't fit :< (%u into %u)\n", (unsigned) len, (unsigned) c.len);
		goto fail;
	}
	printf("does fit :> (%u into %u)\n", (unsigned) len, (unsigned) c.len);
	context2.src = context->src;
	context2.len = len;
	if(!(node = node_new(type, &BLOB_PEEK(context), 0))
		|| !(children = do_parse(&context2, n - 1, ap)))
		goto fail;
	if(context2.len > 0)
	{
		printf("package begun at %p ended early (%u bytes remaining)\n",
			c.src, (unsigned) context2.len);
		print_memory(c.src, 16);
		kernel_loop();
	}
	BLOB_CONSUME(&c, total_len);
	BLOB_COPY(&c, context);
	nod->next = children;
	node_add_child(node, nod);
	printf("getting out of package\n");
	return node;

fail:
	BLOB_COPY(&c, context);
	node_free(nod);
	node_free(node);
	return NULL;
}

aml_node_t *parse_serie(aml_parse_context_t *context, const size_t n, ...)
{
	va_list ap;

	va_start(ap, n);
	return do_parse(context, n, ap);
}

aml_node_t *parse_list(const enum node_type type, aml_parse_context_t *context,
	const parse_func_t f)
{
	aml_node_t *node, *n, *prev, *nod;

	if(!(node = node_new(type, &BLOB_PEEK(context), 0)))
		return NULL;
	prev = node;
	while((n = f(context)))
	{
		if(!(nod = node_new(type, &BLOB_PEEK(context), 0)))
		{
			ast_free(n);
			ast_free(node);
			return NULL;
		}
		node_add_child(nod, n);
		node_add_child(prev, nod);
		prev = nod;
	}
	return node;
}

aml_node_t *parse_fixed_list(const enum node_type type,
	aml_parse_context_t *context, parse_func_t f, size_t i)
{
	aml_node_t *node, *n, *prev, *nod;

	if(!(node = node_new(type, &BLOB_PEEK(context), 0)))
		return NULL;
	prev = node;
	while(i-- > 0 && (n = f(context)))
	{
		if(!(nod = node_new(type, NULL, 0)))
		{
			node_free(n);
			ast_free(node);
			return NULL;
		}
		node_add_child(nod, n);
		node_add_child(prev, nod);
		prev = nod;
	}
	// TODO Check for error
	return node;
}

aml_node_t *parse_string(aml_parse_context_t *context, size_t str_len,
	const parse_func_t f)
{
	aml_node_t *node, *children = NULL, *last_child = NULL;

	while(str_len-- > 0)
	{
		if(!(node = f(context)))
			goto fail;
		if(!last_child)
			last_child = children = node;
		else
		{
			last_child->next = node;
			last_child = node;
		}
		if(!*(node->data))
			break;
	}
	return children;

fail:
	ast_free(children);
	return NULL;
}

aml_node_t *parse_either(const enum node_type type,
	aml_parse_context_t *context, size_t n, ...)
{
	va_list ap;
	aml_parse_context_t c;
	aml_node_t *node, *child;

	va_start(ap, n);
	BLOB_COPY(context, &c);
	if(!(node = node_new(type, &BLOB_PEEK(context), 0)))
		return NULL;
	while(n-- > 0 && !(child = va_arg(ap, parse_func_t)(context)))
		if(errno)
			goto fail;
	if(!child)
		goto fail;
	node_add_child(node, child);
	return node;

fail:
	BLOB_COPY(&c, context);
	node_free(node);
	return NULL;
}

aml_node_t *parse_operation(const int ext_op, const char op,
	const enum node_type type, aml_parse_context_t *context,
		const size_t n, ...)
{
	aml_parse_context_t c;
	va_list ap;
	aml_node_t *children, *node;

	BLOB_COPY(context, &c);
	if(ext_op && !BLOB_CHECK(context, EXT_OP_PREFIX))
		return NULL;
	if(!BLOB_CHECK(context, op) || BLOB_EMPTY(context))
	{
		BLOB_COPY(&c, context);
		return NULL;
	}
	va_start(ap, n);
	if(!(node = node_new(type, &BLOB_PEEK(context), 0))
		|| !(children = do_parse(context, n, ap)))
	{
		BLOB_COPY(&c, context);
		node_free(node);
		return NULL;
	}
	node_add_child(node, children);
	return node;
}

aml_node_t *node_new(const enum node_type type, const char *data,
	const size_t length)
{
	aml_node_t *node;
	char *buff;

	if(!(node = kmalloc_zero(sizeof(aml_node_t), 0)))
		return NULL;
	node->type = type;
	node->ptr = data;
	if(!data || length <= 0)
		return node;
	if(!(buff = kmalloc(length, 0)))
	{
		kfree((void *) node, 0);
		return NULL;
	}
	memcpy(buff, data, length);
	node->data = buff;
	node->data_length = length;
	return node;
}

void node_add_child(aml_node_t *node, aml_node_t *child)
{
	aml_node_t *n;

	if(!node || !child)
		return;
	if((n = node->children))
	{
		while(n->next)
			n = n->next;
		n->next = child;
	}
	else
		node->children = child;
	child->parent = node;
}

#ifdef KERNEL_DEBUG
static void print_indent(size_t n)
{
	while(n--)
		printf(" ");
}

static void ast_print_(const aml_node_t *ast, const size_t level)
{
	const aml_node_t *a;

	if(!ast)
		return;
	print_indent(level);
	printf("- %s: ", node_types[ast->type]);
	tty_write(ast->data, ast->data_length, current_tty); // TODO Use printf precision
	printf("\n");
	a = ast->children;
	while(a)
	{
		ast_print_(a, level + 1);
		a = a->next;
	}
}

void ast_print(const aml_node_t *ast)
{
	ast_print_(ast, 0);
}
#endif

void node_free(aml_node_t *node)
{
	if(!node)
		return;
	kfree((void *) node->data, 0);
	kfree((void *) node, 0);
}

void ast_free(aml_node_t *ast)
{
	if(!ast)
		return;
	ast_free(ast->next);
	ast_free(ast->children);
	node_free(ast);
}
