#include <acpi/aml/aml_parser.h>

static void insert_path_seg(aml_method_path_seg_t **segs, const char *name)
{
	aml_method_path_seg_t *s;

	if(!name || !(s = kmalloc_zero(sizeof(aml_method_path_seg_t), 0)))
		return;
	s->name = name;
	s->next = *segs;
	*segs = s;
}

static void method_set_path(aml_method_t *method, const aml_node_t *node)
{
	while(node)
	{
		if(node->type == AML_DEF_METHOD || node->type == AML_DEF_SCOPE)
			insert_path_seg(&method->path,
				aml_name_string_get(node->children->next)); // TODO Check for NULL?
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
