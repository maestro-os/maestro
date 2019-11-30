#include <acpi/aml/aml_parser.h>

static void insert_path_seg(aml_method_path_seg_t **segs,
	const char *data, const size_t len)
{
	aml_method_path_seg_t *s;

	if(!(s = kmalloc_zero(sizeof(aml_method_path_seg_t), 0)))
		return;
	if(!(s->name = strndup(data, len)))
	{
		kfree(s, 0);
		return;
	}
	s->next = *segs;
	*segs = s;
}

static void method_set_path(aml_method_t *method, const aml_node_t *node)
{
	// TODO If method or scope, call insert_path_seg with the name in argument
	(void) method;
	(void) insert_path_seg;
	while(node)
	{
		if(node->type == AML_DEF_METHOD)
		{
			// TODO
		}
		else if(node->type == AML_DEF_SCOPE)
		{
			// TODO
		}
		node = node->parent;
	}
}

void aml_method_insert(aml_method_t **methods, const aml_node_t *node)
{
	aml_method_t *m;

	if(!methods || !node)
	{
		errno = EINVAL;
		return;
	}
	if(!(m = kmalloc_zero(sizeof(aml_method_t), 0)))
		return;
	method_set_path(m, node);
	// TODO Set arguments
	m->next = *methods;
	*methods = m;
}

const aml_method_t *aml_method_get(const aml_method_t *methods,
	const char *name)
{
	if(!methods || !name)
	{
		errno = EINVAL;
		return NULL;
	}
	// TODO
	return NULL;
}

void aml_method_free(const aml_method_t *methods)
{
	if(!methods)
	{
		errno = EINVAL;
		return;
	}
	// TODO free all
}
