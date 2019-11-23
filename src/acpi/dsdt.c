#include <acpi/acpi.h>
#include <acpi/aml/aml_parser.h>

static void exec_aml(aml_node_t *ast)
{
	// TODO
	(void) ast;
}

void handle_dsdt(dsdt_t *dsdt)
{
	blob_t blob;
	aml_node_t *ast;

	if(!dsdt || !checksum_check(dsdt, dsdt->header.length))
		return;
	blob.src = dsdt->src;
	blob.len = dsdt->header.length - sizeof(dsdt->header);
	if(!(ast = aml_parse(&blob)))
		PANIC("Failed to parse AML code!\n", 0);
	// TODO ast_print(ast);
	exec_aml(ast);
}
