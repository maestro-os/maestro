#include <acpi/acpi.h>
#include <acpi/aml/aml_parser.h>

void handle_dsdt(dsdt_t *dsdt)
{
	size_t len;
	aml_node_t *ast;

	if(!dsdt || !checksum_check(dsdt, dsdt->header.length))
		return;
	len = dsdt->header.length - sizeof(dsdt->header);
	if(!(ast = aml_parse(dsdt->src, len)))
		PANIC("Failed to parse AML code!\n", 0);
	// TODO ast_print(ast);
	// TODO Use/store ast
	(void) ast;
}
