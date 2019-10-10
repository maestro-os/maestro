#include <acpi/aml/aml_parser.h>
#include <stdarg.h>

static aml_node_t *do_parse(const char **src, size_t *len,
	size_t n, va_list ap)
{
	const char *s;
	size_t l;
	aml_node_t *node, *children = NULL, *last_child = NULL;

	s = *src;
	l = *len;
	while(n-- > 0)
	{
		node = va_arg(ap, parse_func_t)(src, len);
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
	ast_free(children);
	*src = s;
	*len = l;
	return NULL;
}

aml_node_t *parse_node(const char **src, size_t *len, const size_t n, ...)
{
	va_list ap;
	aml_node_t *children, *node = NULL;

	va_start(ap, n);
	if(!(children = do_parse(src, len, n, ap)) || !(node = node_new(NULL, 0)))
	{
		ast_free(children);
		return NULL;
	}
	node->children = children;
	return node;
}

aml_node_t *parse_serie(const char **src, size_t *len, const size_t n, ...)
{
	va_list ap;

	va_start(ap, n);
	return do_parse(src, len, n, ap);
}

aml_node_t *parse_string(const char **src, size_t *len,
	size_t str_len, const parse_func_t f)
{
	aml_node_t *node, *children = NULL, *last_child = NULL;

	while(str_len-- > 0)
	{
		if(!(node = f(src, len)))
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

aml_node_t *parse_either(const char **src, size_t *len, size_t n, ...)
{
	const char *s;
	size_t l;
	va_list ap;
	aml_node_t *node;

	s = *src;
	l = *len;
	va_start(ap, n);
	while(n-- > 0 && !(node = va_arg(ap, parse_func_t)(src, len)))
	{
		if(errno)
		{
			*src = s;
			*len = l;
			return NULL;
		}
	}
	return node;
}

aml_node_t *node_new(const char *data, const size_t length)
{
	aml_node_t *node;

	if(!(node = kmalloc_zero(sizeof(aml_node_t), 0)))
		return NULL;
	if(!(node->data = strndup(data, length)))
	{
		kfree((void *) node, 0);
		return NULL;
	}
	return node;
}

// TODO Add a `last_child` variable into node for fast insertion
void node_add_child(aml_node_t *node, aml_node_t *child)
{
	aml_node_t *n;

	if(!node || !child)
		return;
	if(node->children)
	{
		n = node->children;
		while(n->next)
			n = n->next;
		n->next = child;
	}
	else
		node->children = child;
}

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
