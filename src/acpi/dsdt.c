#include <acpi/acpi.h>
#include <acpi/aml/aml_parser.h>

static void exec_aml(aml_node_t *ast)
{
	// TODO
	ast_print(ast);
	(void) ast;
}

void handle_dsdt(dsdt_t *dsdt)
{
	size_t len;
	aml_node_t *ast;

	if(!dsdt || !checksum_check(dsdt, dsdt->header.length))
		return;
	len = dsdt->header.length - sizeof(dsdt->header);
	if(!(ast = aml_parse(dsdt->src, len)))
		PANIC("Failed to parse AML code!\n", 0);
	exec_aml(ast);
}
