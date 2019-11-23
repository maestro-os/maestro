#include <acpi/aml/aml_parser.h>

// TODO Move to `util/util.h`?
#define VARG_COUNT(...)			(sizeof(int[] {__VA_ARGS__}) / sizeof(int))

#define NODE_FUNC_NAME(name)	"parse_" ## name ## "_op"

#define OP_CHECK(opcode)\
	if(!BLOB_CHECK(opcode))\
		goto fail;

#define EXT_OP_CHECK(opcode)\
	if(!BLOB_CHECK(EXT_OP_PREFIX) || !BLOB_CHECK(opcode))\
		goto fail;

#define OP_FOOT()\
	fail:\
		BLOB_COPY(&b, blob);\
		return NULL;

#define PARSE_IMPLICIT_OP(opcode, node, name, ...)\
	static aml_node_t *NODE_FUNC_NAME(name)\
	{\
		blob_t b;\
\
		BLOB_COPY(blob, &b)\
		OP_CHECK(opcode)\
		return parse_node(node, blob, VARG_COUNT(__VA_ARGS__), __VA_ARGS__);\
\
		OP_FOOT()\
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

static aml_node_t *parse_op(enum node_type type, const uint8_t op, blob_t *blob)
{
	blob_t b;
	aml_node_t *node;

	BLOB_COPY(blob, &b);
	if(!BLOB_CHECK(blob, op))
		return NULL;
	if(!(node = node_new(type, &BLOB_PEEK(blob), 1)))
		BLOB_COPY(&b, blob);
	return node;
}

aml_node_t *def_break(blob_t *blob)
{
	return parse_op(AML_DEF_BREAK, BREAK_OP, blob);
}

aml_node_t *def_breakpoint(blob_t *blob)
{
	return parse_op(AML_DEF_BREAK_POINT, BREAKPOINT_OP, blob);
}

aml_node_t *def_continue(blob_t *blob)
{
	return parse_op(AML_DEF_CONTINUE, CONTINUE_OP, blob);
}

aml_node_t *def_else(blob_t *blob)
{
	blob_t b;
	aml_node_t *node;

	BLOB_COPY(blob, &b);
	if(!BLOB_CHECK(blob, ELSE_OP))
		return node_new(AML_DEF_ELSE, &BLOB_PEEK(blob), 0);
	if(!(node = parse_explicit(AML_DEF_ELSE, blob, 2, pkg_length, term_list)))
		BLOB_COPY(&b, blob);
	return node;
}

aml_node_t *def_fatal(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_ifelse(blob_t *blob)
{
	return parse_operation(0, IF_OP, AML_DEF_IF_ELSE, blob,
		4, pkg_length, predicate, term_list, def_else);
}

aml_node_t *predicate(blob_t *blob)
{
	return parse_node(AML_PREDICATE, blob, 1, term_arg);
}

aml_node_t *def_load(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_noop(blob_t *blob)
{
	return parse_op(AML_DEF_NOOP, NOOP_OP, blob);
}

aml_node_t *def_notify(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

static aml_node_t *mutex_object(blob_t *blob)
{
	return parse_node(AML_MUTEX_OBJECT, blob, 1, super_name);
}

aml_node_t *def_release(blob_t *blob)
{
	return parse_operation(1, RELEASE_OP, AML_DEF_RELEASE, blob,
		1, mutex_object);
}

aml_node_t *def_reset(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

static aml_node_t *arg_object(blob_t *blob)
{
	return parse_node(AML_ARG_OBJECT, blob, 1, term_arg);
}

aml_node_t *def_return(blob_t *blob)
{
	return parse_operation(0, RETURN_OP, AML_DEF_RETURN, blob,
		1, arg_object);
}

aml_node_t *def_signal(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_sleep(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_stall(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_while(blob_t *blob)
{
	return parse_operation(0, WHILE_OP, AML_DEF_WHILE, blob,
		3, pkg_length, predicate, term_list);
}

aml_node_t *type1_opcode(blob_t *blob)
{
	printf("type1_opcode:\n");
	print_memory(blob->src, 16);
	return parse_either(AML_TYPE1_OPCODE, blob,
		15, def_break, def_breakpoint, def_continue, def_fatal, def_ifelse,
			def_load, def_noop, def_notify, def_release, def_reset, def_return,
				def_signal, def_sleep, def_stall, def_while);
}

static aml_node_t *timeout(blob_t *blob)
{
	return parse_node(AML_DEF_ACQUIRE, blob, 1, word_data);
}

aml_node_t *def_acquire(blob_t *blob)
{
	return parse_operation(1, ACQUIRE_OP, AML_DEF_ACQUIRE, blob,
		2, mutex_object, timeout);
}

aml_node_t *def_add(blob_t *blob)
{
	return parse_operation(0, ADD_OP, AML_DEF_ADD, blob,
		3, operand, operand, target);
}

aml_node_t *def_and(blob_t *blob)
{
	return parse_operation(0, AND_OP, AML_DEF_AND, blob,
		3, operand, operand, target);
}

static aml_node_t *buffer_size(blob_t *blob)
{
	return parse_node(AML_BUFFER_SIZE, blob, 1, term_arg);
}

aml_node_t *def_buffer(blob_t *blob)
{
	blob_t b;
	aml_node_t *node = NULL, *n0 = NULL, *n1 = NULL, *n2 = NULL;
	size_t buff_size;

	BLOB_COPY(blob, &b);
	if(!BLOB_CHECK(blob, BUFFER_OP))
		return NULL;
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

aml_node_t *def_concat(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_concat_res(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_cond_ref_of(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_copy_object(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_decrement(blob_t *blob)
{
	return parse_operation(0, DECREMENT_OP, AML_DEF_DECREMENT, blob,
		1, super_name);
}

aml_node_t *def_deref_of(blob_t *blob)
{
	return parse_operation(0, DEREF_OF_OP, AML_DEF_DEREF_OF, blob,
		1, obj_reference);
}

aml_node_t *def_divide(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_find_set_left_bit(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_find_set_right_bit(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_from_bcd(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_increment(blob_t *blob)
{
	return parse_operation(0, INCREMENT_OP, AML_DEF_INCREMENT, blob,
		1, super_name);
}

static aml_node_t *buff_pkg_str_obj(blob_t *blob)
{
	return parse_node(AML_BUFF_PKG_STR_OBJ, blob, 1, term_arg);
}

static aml_node_t *index_value(blob_t *blob)
{
	return parse_node(AML_INDEX_VALUE, blob, 1, term_arg);
}

aml_node_t *def_index(blob_t *blob)
{
	return parse_operation(0, INDEX_OP, AML_DEF_INDEX, blob,
		3, buff_pkg_str_obj, index_value, target);
}

aml_node_t *def_l_and(blob_t *blob)
{
	return parse_operation(0, L_AND_OP, AML_DEF_L_AND, blob,
		2, operand, operand);
}

aml_node_t *def_l_equal(blob_t *blob)
{
	return parse_operation(0, L_EQUAL_OP, AML_DEF_L_EQUAL, blob,
		2, operand, operand);
}

aml_node_t *def_l_greater(blob_t *blob)
{
	return parse_operation(0, L_GREATER_OP, AML_DEF_L_GREATER, blob,
		2, operand, operand);
}

aml_node_t *def_l_greater_equal(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_l_less(blob_t *blob)
{
	return parse_operation(0, L_LESS_OP, AML_DEF_L_LESS, blob,
		2, operand, operand);
}

aml_node_t *def_l_less_equal(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_mid(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_l_not(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_l_not_equal(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_load_table(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_l_or(blob_t *blob)
{
	return parse_operation(0, L_OR_OP, AML_DEF_L_OR, blob,
		2, operand, operand);
}

aml_node_t *def_match(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_mod(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_multiply(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_n_and(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_n_or(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_not(blob_t *blob)
{
	return parse_operation(0, NOT_OP, AML_DEF_NOT, blob,
		2, operand, target);
}

aml_node_t *def_object_type(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_or(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
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

aml_node_t *def_package(blob_t *blob)
{
	return parse_operation(0, PACKAGE_OP, AML_DEF_PACKAGE, blob,
		3, pkg_length, num_elements, package_element_list);
}

static aml_node_t *var_num_elements(blob_t *blob)
{
	return parse_node(AML_VAR_NUM_ELEMENTS, blob, 1, term_arg);
}

aml_node_t *def_var_package(blob_t *blob)
{
	return parse_operation(0, VAR_PACKAGE_OP, AML_DEF_VAR_PACKAGE, blob,
		3, pkg_length, var_num_elements, package_element_list);
}

aml_node_t *def_ref_of(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

static aml_node_t *shift_count(blob_t *blob)
{
	return parse_node(AML_SHIFT_COUNT, blob, 1, term_arg);
}

aml_node_t *def_shift_left(blob_t *blob)
{
	return parse_operation(0, SHIFT_LEFT_OP, AML_DEF_SHIFT_LEFT, blob,
		3, operand, shift_count, target);
}

aml_node_t *def_shift_right(blob_t *blob)
{
	return parse_operation(0, SHIFT_RIGHT_OP, AML_DEF_SHIFT_RIGHT, blob,
		3, operand, shift_count, target);
}

aml_node_t *def_size_of(blob_t *blob)
{
	return parse_operation(0, SIZE_OF_OP, AML_DEF_SIZE_OF, blob, 1, super_name);
}

aml_node_t *def_store(blob_t *blob)
{
	return parse_operation(0, STORE_OP, AML_DEF_STORE, blob,
		2, term_arg, super_name);
}

aml_node_t *def_subtract(blob_t *blob)
{
	return parse_operation(0, SUBTRACT_OP, AML_DEF_SUBTRACT, blob,
		3, operand, operand, target);
}

aml_node_t *def_timer(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_to_bcd(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_to_buffer(blob_t *blob)
{
	return parse_operation(0, TO_BUFFER_OP, AML_DEF_TO_BUFFER, blob,
		2, operand, target);
}

aml_node_t *def_to_decimal_string(blob_t *blob)
{
	return parse_operation(0, TO_DECIMAL_STRING_OP, AML_DEF_TO_DECIMAL_STRING,
		blob, 2, operand, target);
}

aml_node_t *def_to_hex_string(blob_t *blob)
{
	return parse_operation(0, TO_HEX_STRING_OP, AML_DEF_TO_HEX_STRING,
		blob, 2, operand, target);
}

aml_node_t *def_to_integer(blob_t *blob)
{
	return parse_operation(0, TO_INTEGER_OP, AML_DEF_TO_INTEGER, blob,
		2, operand, target);
}

aml_node_t *def_to_string(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_wait(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *def_xor(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}

aml_node_t *type2_opcode(blob_t *blob)
{
	static struct
	{
		char ext_prefix;
		const uint8_t op;
		parse_func_t func;
	} funcs[] = {
		{1, ACQUIRE_OP, def_acquire},
		{0, ADD_OP, def_add},
		{0, AND_OP, def_and},
		{0, BUFFER_OP, def_buffer},
		{0, CONCAT_OP, def_concat},
		{0, CONCAT_RES_OP, def_concat_res},
		{1, COND_REF_OF_OP, def_cond_ref_of},
		{0, COPY_OBJECT_OP, def_copy_object},
		{0, DECREMENT_OP, def_decrement},
		{0, DEREF_OF_OP, def_deref_of},
		{0, DIVIDE_OP, def_divide},
		{0, FIND_SET_LEFT_BIT_OP, def_find_set_left_bit},
		{0, FIND_SET_RIGHT_BIT_OP, def_find_set_right_bit},
		{1, FROM_BCD_OP, def_from_bcd},
		{0, INCREMENT_OP, def_increment},
		{0, INDEX_OP, def_index},
		{0, L_AND_OP, def_l_and},
		{0, L_EQUAL_OP, def_l_equal},
		{0, L_GREATER_OP, def_l_greater},
		{0, 0x00, def_l_greater_equal}, // TODO Not op
		{0, L_LESS_OP, def_l_less},
		{0, 0x00, def_l_less_equal}, // TODO Not op
		{0, MID_OP, def_mid}, // TODO
		{0, L_NOT_OP, def_l_not},
		{0, 0x00, def_l_not_equal}, // TODO Not op
		{1, LOAD_TABLE_OP, def_load_table},
		{0, L_OR_OP, def_l_or},
		{0, MATCH_OP, def_match},
		{0, MOD_OP, def_mod},
		{0, MULTIPLY_OP, def_multiply},
		{0, N_AND_OP, def_n_and},
		{0, N_OR_OP, def_n_or},
		{0, NOT_OP, def_not},
		{0, OBJECT_TYPE_OP, def_object_type},
		{0, OR_OP, def_or},
		{0, PACKAGE_OP, def_package},
		{0, VAR_PACKAGE_OP, def_var_package},
		{0, REF_OF_OP, def_ref_of},
		{0, SHIFT_LEFT_OP, def_shift_left},
		{0, SHIFT_RIGHT_OP, def_shift_right},
		{0, SIZE_OF_OP, def_size_of},
		{0, STORE_OP, def_store},
		{0, SUBTRACT_OP, def_subtract},
		{1, TIMER_OP, def_timer},
		{1, TO_BCD_OP, def_to_bcd},
		{0, TO_BUFFER_OP, def_to_buffer},
		{0, TO_DECIMAL_STRING_OP, def_to_decimal_string},
		{0, TO_HEX_STRING_OP, def_to_hex_string},
		{0, TO_INTEGER_OP, def_to_integer},
		{0, TO_STRING_OP, def_to_string},
		{1, WAIT_OP, def_wait},
		{0, XOR_OP, def_xor}
	};
	blob_t b;
	int ext_prefix;
	uint8_t opcode;
	size_t i;

	if(BLOB_EMPTY(blob))
		return NULL;
	BLOB_COPY(blob, &b);
	if((ext_prefix = BLOB_CHECK(blob, EXT_OP_PREFIX)))
	{
		if(BLOB_EMPTY(blob))
		{
			BLOB_COPY(&b, blob);
			return NULL;
		}
	}
	opcode = BLOB_PEEK(blob);
	BLOB_CONSUME(blob, 1);
	for(i = 0; i < sizeof(funcs) / sizeof(*funcs); ++i)
	{
		if(ext_prefix != funcs[i].ext_prefix)
			continue;
		if(opcode != funcs[i].op)
			continue;
		return parse_node(AML_TYPE2_OPCODE, blob, 1, funcs[i].func);
	}
	return parse_node(AML_TYPE2_OPCODE, blob, 1, method_invocation);
}

aml_node_t *type6_opcode(blob_t *blob)
{
	// TODO
	(void) blob;
	return NULL;
}
