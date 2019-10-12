#include <acpi/aml/aml_parser.h>

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
	return parse_op(DEF_BREAK, BREAK_OP, src, len);
}

aml_node_t *def_breakpoint(const char **src, size_t *len)
{
	return parse_op(DEF_BREAK_POINT, BREAKPOINT_OP, src, len);
}

aml_node_t *def_continue(const char **src, size_t *len)
{
	return parse_op(DEF_CONTINUE, CONTINUE_OP, src, len);
}

aml_node_t *def_else(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
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
	const char *s;
	size_t l;
	aml_node_t *node;

	if(*len < 1 || (uint8_t) **src != IF_OP)
		return NULL;
	s = *src;
	l = *len;
	if(!(node = parse_node(DEF_IF_ELSE, src, len,
		4, pkg_length, predicate, term_list, def_else)))
	{
		*src = s;
		*len = l;
		return NULL;
	}
	return node;
}

aml_node_t *predicate(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
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
	return parse_op(DEF_NOOP, NOOP_OP, src, len);
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

aml_node_t *def_return(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
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
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *type1_opcode(const char **src, size_t *len)
{
	return parse_either(src, len, 15, def_break, def_breakpoint, def_continue,
		def_fatal, def_ifelse, def_load, def_noop, def_notify, def_release,
			def_reset, def_return, def_signal, def_sleep, def_stall, def_while);
}

aml_node_t *type2_opcode(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}
