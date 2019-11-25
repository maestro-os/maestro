#include <acpi/aml/aml_parser.h>

#define NODE_FUNC_NAME(name)	def_ ## name

#define OP_CHECK_MACRO(ext)		OP_CHECK_ ## ext
#define OP_CHECK____(opcode)\
if(!BLOB_CHECK(blob, opcode))\
	return NULL;
#define OP_CHECK_EXT(opcode)\
if(!BLOB_CHECK(blob, EXT_OP_PREFIX) || !BLOB_CHECK(blob, opcode))\
{\
	BLOB_COPY(&b, blob);\
	return NULL;\
}\

// TODO remove debug lines
#define OP_HEAD(ext, opcode, name)\
	blob_t b;\
	aml_node_t *n;\
\
	printf("test opcode -> %s\n", #name);\
	print_memory(blob->src, 16);\
	BLOB_COPY(blob, &b);\
	OP_CHECK_MACRO(ext)(opcode)\
	printf("opcode -> %s\n", #name);

#define PARSE_EMPTY_OP(ext, opcode, node, name)\
aml_node_t *NODE_FUNC_NAME(name)(blob_t *blob)\
{\
	OP_HEAD(ext, opcode, name)\
	if(!(n = parse_node(node, blob, 0)))\
	{\
		BLOB_COPY(&b, blob);\
		return NULL;\
	}\
	return n;\
}

#define PARSE_IMPLICIT_OP(ext, opcode, node, name, ...)\
aml_node_t *NODE_FUNC_NAME(name)(blob_t *blob)\
{\
	OP_HEAD(ext, opcode, name)\
	if(!(n = parse_node(node, blob,\
		VARG_COUNT(__VA_ARGS__), __VA_ARGS__)))\
	{\
		BLOB_COPY(&b, blob);\
		return NULL;\
	}\
	return n;\
}

#define PARSE_EXPLICIT_OP(ext, opcode, node, name, ...)\
aml_node_t *NODE_FUNC_NAME(name)(blob_t *blob)\
{\
	OP_HEAD(ext, opcode, name)\
	if(!(n = parse_explicit(node, blob,\
		VARG_COUNT(__VA_ARGS__), __VA_ARGS__)))\
	{\
		BLOB_COPY(&b, blob);\
		return NULL;\
	}\
	return n;\
}

// TODO remove
#define TODO_OP(ext, opcode, node, name)\
aml_node_t *NODE_FUNC_NAME(name)(blob_t *blob)\
{\
	(void) blob;\
	return NULL;\
}

typedef struct
{
	char ext_prefix;
	const uint8_t op;
	parse_func_t func;
} op_descriptor_t;

static aml_node_t *parse_opcode(blob_t *blob, enum node_type type,
	op_descriptor_t *ops, const size_t ops_count)
{
	blob_t b;
	int ext_prefix;
	uint8_t opcode;
	size_t i;

	if(BLOB_EMPTY(blob))
		return NULL;
	BLOB_COPY(blob, &b);
	if((ext_prefix = (blob->src[0] == EXT_OP_PREFIX)))
	{
		if(BLOB_EMPTY(blob))
		{
			BLOB_COPY(&b, blob);
			return NULL;
		}
		opcode = blob->src[1];
	}
	else
		opcode = blob->src[0];
	for(i = 0; i < ops_count; ++i)
	{
		if(ext_prefix != ops[i].ext_prefix)
			continue;
		if(opcode != ops[i].op)
			continue;
		// TODO End parsing if fail here? (we are sure that this is that operation at this moment)
		// TODO Must put blob back to original state on fail?
		return parse_node(type, blob, 1, ops[i].func);
	}
	BLOB_COPY(&b, blob);
	return NULL;
}

static aml_node_t *operand(blob_t *blob)
{
	return parse_node(AML_OPERAND, blob, 1, term_arg);
}

static aml_node_t *target(blob_t *blob)
{
	return parse_either(AML_TARGET, blob, 2, super_name, null_name);
}

aml_node_t *obj_reference(blob_t *blob)
{
	return parse_either(AML_OBJ_REFERENCE, blob, 2, term_arg, string);
}

aml_node_t *predicate(blob_t *blob)
{
	return parse_node(AML_PREDICATE, blob, 1, term_arg);
}

static aml_node_t *mutex_object(blob_t *blob)
{
	return parse_node(AML_MUTEX_OBJECT, blob, 1, super_name);
}

static aml_node_t *arg_object(blob_t *blob)
{
	return parse_node(AML_ARG_OBJECT, blob, 1, term_arg);
}

PARSE_EMPTY_OP(___, BREAK_OP, AML_DEF_BREAK, break)
PARSE_EMPTY_OP(___, BREAKPOINT_OP, AML_DEF_BREAK_POINT, breakpoint)
PARSE_EMPTY_OP(___, CONTINUE_OP, AML_DEF_CONTINUE, continue)
PARSE_EXPLICIT_OP(___, ELSE_OP, AML_DEF_ELSE, else_, pkg_length, term_list)

aml_node_t *def_else(blob_t *blob)
{
	if(BLOB_EMPTY(blob) || BLOB_PEEK(blob) != ELSE_OP)
		return node_new(AML_DEF_ELSE, &BLOB_PEEK(blob), 0);
	return NODE_FUNC_NAME(else_)(blob);
}

TODO_OP(EXT, FATAL_OP, AML_DEF_FATAL, fatal) // TODO
PARSE_EXPLICIT_OP(___, IF_OP, AML_DEF_IF_ELSE, ifelse,
	pkg_length, predicate, term_list, def_else)
TODO_OP(EXT, LOAD_OP, AML_DEF_LOAD, load) // TODO
PARSE_EMPTY_OP(___, NOOP_OP, AML_DEF_NOOP, noop)
TODO_OP(___, NOTIFY_OP, AML_DEF_NOTIFY, notify) // TODO
PARSE_IMPLICIT_OP(EXT, RELEASE_OP, AML_DEF_RELEASE, release, mutex_object)
TODO_OP(EXT, RESET_OP, AML_DEF_RESET, reset) // TODO
PARSE_IMPLICIT_OP(___, RETURN_OP, AML_DEF_RETURN, return, arg_object)
TODO_OP(EXT, SIGNAL_OP, AML_DEF_SIGNAL, signal) // TODO
TODO_OP(EXT, SLEEP_OP, AML_DEF_SLEEP, sleep) // TODO
TODO_OP(EXT, STALL_OP, AML_DEF_STALL, stall) // TODO
PARSE_EXPLICIT_OP(___, WHILE_OP, AML_DEF_WHILE, while,
	pkg_length, predicate, term_list)

static op_descriptor_t type1_ops[] = {
	{0, BREAK_OP, NODE_FUNC_NAME(break)},
	{0, BREAKPOINT_OP, NODE_FUNC_NAME(breakpoint)},
	{0, CONTINUE_OP, NODE_FUNC_NAME(continue)},
	{0, ELSE_OP, NODE_FUNC_NAME(else)},
	{1, FATAL_OP, NODE_FUNC_NAME(fatal)},
	{0, IF_OP, NODE_FUNC_NAME(ifelse)},
	{1, LOAD_OP, NODE_FUNC_NAME(load)},
	{0, NOOP_OP, NODE_FUNC_NAME(noop)},
	{0, NOTIFY_OP, NODE_FUNC_NAME(notify)},
	{1, RELEASE_OP, NODE_FUNC_NAME(release)},
	{1, RESET_OP, NODE_FUNC_NAME(reset)},
	{0, RETURN_OP, NODE_FUNC_NAME(return)},
	{1, SIGNAL_OP, NODE_FUNC_NAME(signal)},
	{1, SLEEP_OP, NODE_FUNC_NAME(sleep)},
	{1, STALL_OP, NODE_FUNC_NAME(stall)},
	{0, WHILE_OP, NODE_FUNC_NAME(while)}
};

aml_node_t *type1_opcode(blob_t *blob)
{
	return parse_opcode(blob, AML_TYPE1_OPCODE,
		type1_ops, sizeof(type1_ops) / sizeof(*type1_ops));
}

static aml_node_t *timeout(blob_t *blob)
{
	return parse_node(AML_DEF_ACQUIRE, blob, 1, word_data);
}

static aml_node_t *buffer_size(blob_t *blob)
{
	return parse_node(AML_BUFFER_SIZE, blob, 1, term_arg);
}

static aml_node_t *buff_pkg_str_obj(blob_t *blob)
{
	return parse_node(AML_BUFF_PKG_STR_OBJ, blob, 1, term_arg);
}

static aml_node_t *index_value(blob_t *blob)
{
	return parse_node(AML_INDEX_VALUE, blob, 1, term_arg);
}

static aml_node_t *num_elements(blob_t *blob)
{
	return parse_node(AML_NUM_ELEMENTS, blob, 1, byte_data);
}

static aml_node_t *package_element(blob_t *blob)
{
	return parse_either(AML_PACKAGE_ELEMENT, blob,
		2, data_ref_object, name_string);
}

static aml_node_t *package_element_list(blob_t *blob)
{
	return parse_list(AML_PACKAGE_ELEMENT_LIST, blob, package_element);
}

static aml_node_t *var_num_elements(blob_t *blob)
{
	return parse_node(AML_VAR_NUM_ELEMENTS, blob, 1, term_arg);
}

static aml_node_t *shift_count(blob_t *blob)
{
	return parse_node(AML_SHIFT_COUNT, blob, 1, term_arg);
}

PARSE_IMPLICIT_OP(EXT, ACQUIRE_OP, AML_DEF_ACQUIRE, acquire,
	mutex_object, timeout)
PARSE_IMPLICIT_OP(___, ADD_OP, AML_DEF_ADD, add, operand, operand, target)
PARSE_IMPLICIT_OP(___, AND_OP, AML_DEF_AND, and, operand, operand, target)

// TODO explicit length
aml_node_t *NODE_FUNC_NAME(buffer)(blob_t *blob)
{
	blob_t b;
	aml_node_t *node = NULL, *n0 = NULL, *n1 = NULL, *n2 = NULL;
	size_t buff_size;

	BLOB_COPY(blob, &b);
	if(!(node = node_new(AML_DEF_BUFFER, &BLOB_PEEK(blob), 0)))
		goto fail;
	if(!(n0 = pkg_length(blob)))
		goto fail;
	if(!(n1 = buffer_size(blob)))
		goto fail;
	buff_size = aml_get_integer(n1->children);
	if(!(n2 = byte_list(blob, buff_size)))
		goto fail;
	node_add_child(node, n0);
	node_add_child(node, n1);
	node_add_child(node, n2);
	return node;

fail:
	BLOB_COPY(&b, blob);
	ast_free(n0);
	ast_free(n1);
	ast_free(n2);
	ast_free(node);
	return NULL;
}

TODO_OP(___, CONCAT_OP, AML_DEF_CONCAT, concat) // TODO
TODO_OP(___, CONCAT_RES_OP, AML_DEF_CONCAT_RES, concat_res) // TODO
TODO_OP(___, COND_REF_OF_OP, AML_DEF_COND_REF_OF, cond_ref_of) // TODO
TODO_OP(___, COPY_OBJECT_OP, AML_DEF_COPY_OBJECT, copy_object) // TODO

PARSE_IMPLICIT_OP(___, DECREMENT_OP, AML_DEF_DECREMENT, decrement, super_name)
PARSE_IMPLICIT_OP(___, DEREF_OF_OP, AML_DEF_DEREF_OF, deref_of, obj_reference)

TODO_OP(___, DIVIDE_OP, AML_DEF_DIVIDE, divide) // TODO
TODO_OP(___, FIND_SET_LEFT_BIT_OP, AML_DEF_FIND_SET_LEFT_BIT, find_set_left_bit) // TODO
TODO_OP(___, FIND_SET_RIGHT_BIT_OP, AML_DEF_FIND_SET_RIGHT_BIT, find_set_right_bit) // TODO
TODO_OP(EXT, FROM_BCD_OP, AML_DEF_FROM_BCD, from_bcd) // TODO

PARSE_IMPLICIT_OP(___, INCREMENT_OP, AML_DEF_INCREMENT, increment, super_name)
PARSE_IMPLICIT_OP(___, INDEX_OP, AML_DEF_INDEX, index,
	buff_pkg_str_obj, index_value, target)
PARSE_IMPLICIT_OP(___, L_AND_OP, AML_DEF_L_AND, l_and, operand, operand)
PARSE_IMPLICIT_OP(___, L_EQUAL_OP, AML_DEF_L_EQUAL, l_equal, operand, operand)
PARSE_IMPLICIT_OP(___, L_GREATER_OP, AML_DEF_L_GREATER, l_greater,
	operand, operand)
TODO_OP(___, L_GREATER_EQUAL_OP, AML_DEF_L_GREATER_EQUAL, l_greater_equal) // TODO
PARSE_IMPLICIT_OP(___, L_LESS_OP, AML_DEF_L_LESS, l_less, operand, operand)
TODO_OP(___, L_LESS_EQUAL_OP, AML_DEF_L_LESS_EQUAL, l_less_equal) // TODO
TODO_OP(___, MID_OP, AML_DEF_MID, mid) // TODO
TODO_OP(___, L_NOT_OP, AML_DEF_L_NOT, l_not) // TODO
TODO_OP(___, L_NOT_EQUAL_OP, AML_DEF_L_NOT_EQUAL, l_not_equal) // TODO
TODO_OP(EXT, LOAD_TABLE_OP, AML_DEF_LOAD_TABLE, load_table) // TODO
PARSE_IMPLICIT_OP(___, L_OR_OP, AML_DEF_L_OR, l_or, operand, operand)
TODO_OP(___, MATCH_OP, AML_DEF_MATCH, match) // TODO
TODO_OP(___, MOD_OP, AML_DEF_MOD, mod) // TODO
TODO_OP(___, MULTIPLY_OP, AML_DEF_MULTIPLY, multiply) // TODO
TODO_OP(___, N_AND_OP, AML_DEF_N_AND, n_and) // TODO
TODO_OP(___, N_OR_OP, AML_DEF_N_OR, n_or) // TODO
PARSE_IMPLICIT_OP(___, NOT_OP, AML_DEF_NOT, not, operand, target)
TODO_OP(___, OBJECT_TYPE_OP, AML_DEF_OBJECT_TYPE, object_type) // TODO
TODO_OP(___, OR_OP, AML_DEF_OR, or) // TODO
PARSE_EXPLICIT_OP(___, PACKAGE_OP, AML_DEF_PACKAGE, package,
	pkg_length, num_elements, package_element_list)
PARSE_EXPLICIT_OP(___, VAR_PACKAGE_OP, AML_DEF_VAR_PACKAGE, var_package,
	pkg_length, var_num_elements, package_element_list)
TODO_OP(___, DEF_REF_OF_OP, AML_DEF_DEF_REF_OF, ref_of) // TODO
PARSE_IMPLICIT_OP(___, SHIFT_LEFT_OP, AML_DEF_SHIFT_LEFT, shift_left,
	operand, shift_count, target)
PARSE_IMPLICIT_OP(___, SHIFT_RIGHT_OP, AML_DEF_SHIFT_RIGHT, shift_right,
	operand, shift_count, target)
PARSE_IMPLICIT_OP(___, SIZE_OF_OP, AML_DEF_SIZE_OF, size_of, super_name)
PARSE_IMPLICIT_OP(___, STORE_OP, AML_DEF_STORE, store, term_arg, super_name)
PARSE_IMPLICIT_OP(___, SUBTRACT_OP, AML_DEF_SUBTRACT, subtract,
	operand, operand, target)
TODO_OP(EXT, TIMER_OP, AML_DEF_TIMER, timer) // TODO
TODO_OP(EXT, TO_BCD_OP, AML_DEF_TO_BCD, to_bcd) // TODO
PARSE_IMPLICIT_OP(___, TO_BUFFER_OP, AML_DEF_TO_BUFFER, to_buffer, operand, target)
PARSE_IMPLICIT_OP(___, TO_DECIMAL_STRING_OP, AML_DEF_TO_DECIMAL_STRING,
	to_decimal_string, operand, target)
PARSE_IMPLICIT_OP(___, TO_HEX_STRING_OP, AML_DEF_TO_HEX_STRING, to_hex_string,
	operand, target)
PARSE_IMPLICIT_OP(___, TO_INTEGER_OP, AML_DEF_TO_INTEGER, to_integer,
	operand, target)
TODO_OP(___, TO_STRING_OP, AML_DEF_TO_STRING, to_string) // TODO
TODO_OP(EXT, WAIT_OP, AML_DEF_WAIT, wait) // TODO
TODO_OP(___, XOR_OP, AML_DEF_XOR, xor) // TODO

static op_descriptor_t type2_ops[] = {
	{1, ACQUIRE_OP, NODE_FUNC_NAME(acquire)},
	{0, ADD_OP, NODE_FUNC_NAME(add)},
	{0, AND_OP, NODE_FUNC_NAME(and)},
	{0, BUFFER_OP, NODE_FUNC_NAME(buffer)},
	{0, CONCAT_OP, NODE_FUNC_NAME(concat)},
	{0, CONCAT_RES_OP, NODE_FUNC_NAME(concat_res)},
	{1, COND_REF_OF_OP, NODE_FUNC_NAME(cond_ref_of)},
	{0, COPY_OBJECT_OP, NODE_FUNC_NAME(copy_object)},
	{0, DECREMENT_OP, NODE_FUNC_NAME(decrement)},
	{0, DEREF_OF_OP, NODE_FUNC_NAME(deref_of)},
	{0, DIVIDE_OP, NODE_FUNC_NAME(divide)},
	{0, FIND_SET_LEFT_BIT_OP, NODE_FUNC_NAME(find_set_left_bit)},
	{0, FIND_SET_RIGHT_BIT_OP, NODE_FUNC_NAME(find_set_right_bit)},
	{1, FROM_BCD_OP, NODE_FUNC_NAME(from_bcd)},
	{0, INCREMENT_OP, NODE_FUNC_NAME(increment)},
	{0, INDEX_OP, NODE_FUNC_NAME(index)},
	{0, L_AND_OP, NODE_FUNC_NAME(l_and)},
	{0, L_EQUAL_OP, NODE_FUNC_NAME(l_equal)},
	{0, L_GREATER_OP, NODE_FUNC_NAME(l_greater)},
	{0, 0x00, NODE_FUNC_NAME(l_greater_equal)}, // TODO Not op
	{0, L_LESS_OP, NODE_FUNC_NAME(l_less)},
	{0, 0x00, NODE_FUNC_NAME(l_less_equal)}, // TODO Not op
	{0, MID_OP, NODE_FUNC_NAME(mid)}, // TODO
	{0, L_NOT_OP, NODE_FUNC_NAME(l_not)},
	{0, 0x00, NODE_FUNC_NAME(l_not_equal)}, // TODO Not op
	{1, LOAD_TABLE_OP, NODE_FUNC_NAME(load_table)},
	{0, L_OR_OP, NODE_FUNC_NAME(l_or)},
	{0, MATCH_OP, NODE_FUNC_NAME(match)},
	{0, MOD_OP, NODE_FUNC_NAME(mod)},
	{0, MULTIPLY_OP, NODE_FUNC_NAME(multiply)},
	{0, N_AND_OP, NODE_FUNC_NAME(n_and)},
	{0, N_OR_OP, NODE_FUNC_NAME(n_or)},
	{0, NOT_OP, NODE_FUNC_NAME(not)},
	{0, OBJECT_TYPE_OP, NODE_FUNC_NAME(object_type)},
	{0, OR_OP, NODE_FUNC_NAME(or)},
	{0, PACKAGE_OP, NODE_FUNC_NAME(package)},
	{0, VAR_PACKAGE_OP, NODE_FUNC_NAME(var_package)},
	{0, REF_OF_OP, NODE_FUNC_NAME(ref_of)},
	{0, SHIFT_LEFT_OP, NODE_FUNC_NAME(shift_left)},
	{0, SHIFT_RIGHT_OP, NODE_FUNC_NAME(shift_right)},
	{0, SIZE_OF_OP, NODE_FUNC_NAME(size_of)},
	{0, STORE_OP, NODE_FUNC_NAME(store)},
	{0, SUBTRACT_OP, NODE_FUNC_NAME(subtract)},
	{1, TIMER_OP, NODE_FUNC_NAME(timer)},
	{1, TO_BCD_OP, NODE_FUNC_NAME(to_bcd)},
	{0, TO_BUFFER_OP, NODE_FUNC_NAME(to_buffer)},
	{0, TO_DECIMAL_STRING_OP, NODE_FUNC_NAME(to_decimal_string)},
	{0, TO_HEX_STRING_OP, NODE_FUNC_NAME(to_hex_string)},
	{0, TO_INTEGER_OP, NODE_FUNC_NAME(to_integer)},
	{0, TO_STRING_OP, NODE_FUNC_NAME(to_string)},
	{1, WAIT_OP, NODE_FUNC_NAME(wait)},
	{0, XOR_OP, NODE_FUNC_NAME(xor)}
};

aml_node_t *type2_opcode(blob_t *blob)
{
	aml_node_t *n;

	if((n = parse_opcode(blob, AML_TYPE2_OPCODE,
		type2_ops, sizeof(type2_ops) / sizeof(*type2_ops))))
		return n;
	return parse_node(AML_TYPE2_OPCODE, blob, 1, method_invocation);
}

aml_node_t *type6_opcode(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}
