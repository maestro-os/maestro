#include <acpi/aml/aml_parser.h>

static aml_node_t *operand(const char **src, size_t *len)
{
	return parse_node(AML_OPERAND, src, len, 1, term_arg);
}

static aml_node_t *target(const char **src, size_t *len)
{
	return parse_either(AML_TARGET, src, len, 2, super_name, null_name);
}

aml_node_t *obj_reference(const char **src, size_t *len)
{
	return parse_either(AML_OBJ_REFERENCE, src, len, 2, term_arg, string);
}

static aml_node_t *parse_op(enum node_type type, const uint8_t op,
	const char **src, size_t *len)
{
	aml_node_t *node;

	if(*len < 1 || **src != op || !(node = node_new(type, *src, 1)))
		return NULL;
	++(*src);
	--(*len);
	return node;
}

aml_node_t *def_break(const char **src, size_t *len)
{
	return parse_op(AML_DEF_BREAK, BREAK_OP, src, len);
}

aml_node_t *def_breakpoint(const char **src, size_t *len)
{
	return parse_op(AML_DEF_BREAK_POINT, BREAKPOINT_OP, src, len);
}

aml_node_t *def_continue(const char **src, size_t *len)
{
	return parse_op(AML_DEF_CONTINUE, CONTINUE_OP, src, len);
}

aml_node_t *def_else(const char **src, size_t *len)
{
	const char *s;
	size_t l;
	aml_node_t *node;

	if(*len < 1 || **src != ELSE_OP)
		return node_new(AML_DEF_ELSE, *src, 0);
	s = (*src)++;
	l = (*len)--;
	if(!(node = parse_node(AML_DEF_ELSE, src, len, 2, pkg_length, term_list)))
	{
		*src = s;
		*len = l;
	}
	return node;
}

aml_node_t *def_fatal(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_ifelse(const char **src, size_t *len)
{
	return parse_operation(0, IF_OP, AML_DEF_IF_ELSE, src, len,
		4, pkg_length, predicate, term_list, def_else);
}

aml_node_t *predicate(const char **src, size_t *len)
{
	return parse_node(AML_PREDICATE, src, len, 1, term_arg);
}

aml_node_t *def_load(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_noop(const char **src, size_t *len)
{
	return parse_op(AML_DEF_NOOP, NOOP_OP, src, len);
}

aml_node_t *def_notify(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_release(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_reset(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

static aml_node_t *arg_object(const char **src, size_t *len)
{
	return parse_node(AML_ARG_OBJECT, src, len, 1, term_arg);
}

aml_node_t *def_return(const char **src, size_t *len)
{
	return parse_operation(0, RETURN_OP, AML_DEF_RETURN, src, len,
		1, arg_object);
}

aml_node_t *def_signal(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_sleep(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_stall(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_while(const char **src, size_t *len)
{
	return parse_operation(0, WHILE_OP, AML_DEF_WHILE, src, len,
		3, pkg_length, predicate, term_list);
}

aml_node_t *type1_opcode(const char **src, size_t *len)
{
	return parse_either(AML_TYPE1_OPCODE, src, len,
		15, def_break, def_breakpoint, def_continue, def_fatal, def_ifelse,
			def_load, def_noop, def_notify, def_release, def_reset, def_return,
				def_signal, def_sleep, def_stall, def_while);
}

aml_node_t *def_acquire(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_add(const char **src, size_t *len)
{
	return parse_operation(0, ADD_OP, AML_DEF_ADD, src, len,
		3, operand, operand, target);
}

aml_node_t *def_and(const char **src, size_t *len)
{
	return parse_operation(0, AND_OP, AML_DEF_AND, src, len,
		3, operand, operand, target);
}

static aml_node_t *buffer_size(const char **src, size_t *len)
{
	return parse_node(AML_BUFFER_SIZE, src, len, 1, term_arg);
}

aml_node_t *def_buffer(const char **src, size_t *len)
{
	const char *s;
	size_t l;
	aml_node_t *node = NULL, *n0 = NULL, *n1 = NULL, *n2 = NULL;
	size_t buff_size;

	if(*len < 1 || **src != BUFFER_OP)
		return NULL;
	s = (*src)++;
	l = (*len)--;
	if(!(node = node_new(AML_DEF_BUFFER, *src, 0)))
		goto fail;
	if(!(n0 = pkg_length(src, len)))
		goto fail;
	if(!(n1 = buffer_size(src, len)))
		goto fail;
	buff_size = aml_get_integer(n1->children);
	if(!(n2 = byte_list(src, len, buff_size)))
		goto fail;
	node_add_child(node, n0);
	node_add_child(node, n1);
	node_add_child(node, n2);
	return node;

fail:
	*src = s;
	*len = l;
	ast_free(n0);
	ast_free(n1);
	ast_free(n2);
	ast_free(node);
	return NULL;
}

aml_node_t *def_concat(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_concat_res(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_cond_ref_of(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_copy_object(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_decrement(const char **src, size_t *len)
{
	return parse_operation(0, DECREMENT_OP, AML_DEF_DECREMENT, src, len,
		1, super_name);
}

aml_node_t *def_deref_of(const char **src, size_t *len)
{
	return parse_operation(0, DEREF_OF_OP, AML_DEF_DEREF_OF, src, len,
		1, obj_reference);
}

aml_node_t *def_divide(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_find_set_left_bit(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_find_set_right_bit(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_from_bcd(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_increment(const char **src, size_t *len)
{
	return parse_operation(0, INCREMENT_OP, AML_DEF_INCREMENT, src, len,
		1, super_name);
}

static aml_node_t *buff_pkg_str_obj(const char **src, size_t *len)
{
	return parse_node(AML_BUFF_PKG_STR_OBJ, src, len, 1, term_arg);
}

static aml_node_t *index_value(const char **src, size_t *len)
{
	return parse_node(AML_INDEX_VALUE, src, len, 1, term_arg);
}

aml_node_t *def_index(const char **src, size_t *len)
{
	return parse_operation(0, INDEX_OP, AML_DEF_INDEX, src, len,
		3, buff_pkg_str_obj, index_value, target);
}

aml_node_t *def_l_and(const char **src, size_t *len)
{
	return parse_operation(0, L_AND_OP, AML_DEF_L_AND, src, len,
		2, operand, operand);
}

aml_node_t *def_l_equal(const char **src, size_t *len)
{
	return parse_operation(0, L_EQUAL_OP, AML_DEF_L_EQUAL, src, len,
		2, operand, operand);
}

aml_node_t *def_l_greater(const char **src, size_t *len)
{
	return parse_operation(0, L_GREATER_OP, AML_DEF_L_GREATER, src, len,
		2, operand, operand);
}

aml_node_t *def_l_greater_equal(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_l_less(const char **src, size_t *len)
{
	return parse_operation(0, L_LESS_OP, AML_DEF_L_LESS, src, len,
		2, operand, operand);
}

aml_node_t *def_l_less_equal(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_mid(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_l_not(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_l_not_equal(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_load_table(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_l_or(const char **src, size_t *len)
{
	return parse_operation(0, L_OR_OP, AML_DEF_L_OR, src, len,
		2, operand, operand);
}

aml_node_t *def_match(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_mod(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_multiply(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_n_and(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_n_or(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_not(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_object_type(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_or(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_package(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_var_package(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_ref_of(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

static aml_node_t *shift_count(const char **src, size_t *len)
{
	return parse_node(AML_SHIFT_COUNT, src, len, 1, term_arg);
}

aml_node_t *def_shift_left(const char **src, size_t *len)
{
	return parse_operation(0, SHIFT_LEFT_OP, AML_DEF_SHIFT_LEFT, src, len,
		3, operand, shift_count, target);
}

aml_node_t *def_shift_right(const char **src, size_t *len)
{
	return parse_operation(0, SHIFT_RIGHT_OP, AML_DEF_SHIFT_RIGHT, src, len,
		3, operand, shift_count, target);
}

aml_node_t *def_size_of(const char **src, size_t *len)
{
	return parse_operation(0, SIZE_OF_OP, AML_DEF_SIZE_OF, src, len,
		1, super_name);
}

aml_node_t *def_store(const char **src, size_t *len)
{
	return parse_operation(0, STORE_OP, AML_DEF_STORE, src, len,
		2, term_arg, super_name);
}

aml_node_t *def_subtract(const char **src, size_t *len)
{
	return parse_operation(0, SUBTRACT_OP, AML_DEF_SUBTRACT, src, len,
		3, operand, operand, target);
}

aml_node_t *def_timer(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_to_bcd(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_to_buffer(const char **src, size_t *len)
{
	return parse_operation(0, TO_BUFFER_OP, AML_DEF_TO_BUFFER, src, len,
		2, operand, target);
}

aml_node_t *def_to_decimal_string(const char **src, size_t *len)
{
	return parse_operation(0, TO_DECIMAL_STRING_OP, AML_DEF_TO_DECIMAL_STRING,
		src, len, 2, operand, target);
}

aml_node_t *def_to_hex_string(const char **src, size_t *len)
{
	return parse_operation(0, TO_HEX_STRING_OP, AML_DEF_TO_HEX_STRING,
		src, len, 2, operand, target);
}

aml_node_t *def_to_integer(const char **src, size_t *len)
{
	return parse_operation(0, TO_INTEGER_OP, AML_DEF_TO_INTEGER, src, len,
		2, operand, target);
}

aml_node_t *def_to_string(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_wait(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_xor(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *type2_opcode(const char **src, size_t *len)
{
	static struct
	{
		char ext_prefix;
		const uint8_t op;
		aml_node_t *(*func)(const char **, size_t *);
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
	int ext_prefix;
	uint8_t opcode;
	size_t i;

	if(*len < 1)
		return NULL;
	if((ext_prefix = (**src == EXT_OP_PREFIX)))
	{
		if(*len < 2)
			return NULL;
		opcode = (*src)[1];
	}
	else
		opcode = (*src)[0];
	for(i = 0; i < sizeof(funcs) / sizeof(*funcs); ++i)
	{
		if(ext_prefix != funcs[i].ext_prefix)
			continue;
		if(opcode != funcs[i].op)
			continue;
		return parse_node(AML_TYPE2_OPCODE, src, len, 1, funcs[i].func);
	}
	return parse_node(AML_TYPE2_OPCODE, src, len, 1, method_invocation);
}

aml_node_t *type6_opcode(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}
