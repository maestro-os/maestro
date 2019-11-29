#include <acpi/aml/aml_parser.h>

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
	m->node = node;
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

void aml_method_free(const aml_method_t **methods)
{
	if(!methods)
	{
		errno = EINVAL;
		return;
	}
	// TODO free all
	*methods = NULL;
}
