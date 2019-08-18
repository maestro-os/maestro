#include <acpi/aml/aml_parser.h>

aml_node_t *def_break(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_breakpoint(const char **src, size_t *len)
{
	// TODO
	(void) src;
	(void) len;
	return NULL;
}

aml_node_t *def_continue(const char **src, size_t *len)
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
	// TODO
	(void) src;
	(void) len;
	return NULL;
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
