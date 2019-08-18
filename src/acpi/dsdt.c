#include <acpi/acpi.h>
#include <acpi/aml/aml_parser.h>

static void exec_aml(aml_node_t *ast)
{
	// TODO
	(void) ast;
}

void handle_dsdt(dsdt_t *dsdt)
{
	size_t len;
	aml_node_t *ast;

	if(!dsdt || !checksum_check(dsdt, dsdt->header.length))
		return;
	len = dsdt->header.length - sizeof(dsdt->header);
	ast = aml_parse(dsdt->src, len);
	exec_aml(ast);
}
