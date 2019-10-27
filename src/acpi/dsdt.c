#include <acpi/acpi.h>
#include <acpi/aml/aml_parser.h>
#include <acpi/aml/aml.h>

// TODO Try loading from file if not correct?

static inline void aml_get_header(aml_node_t *ast, aml_block_header_t *hdr)
{
	memcpy(hdr, ast->children->ptr, sizeof(aml_block_header_t));
}

static int aml_check_header(aml_block_header_t *hdr)
{
	// TODO Check signature?
	if(hdr->table_length != sizeof(aml_block_header_t))
		return 0;
	if(!checksum_check(hdr, hdr->table_length))
		return 0;
	return 1;
}

static void exec_aml(aml_node_t *ast)
{
	// TODO
	(void) ast;
}

void handle_dsdt(dsdt_t *dsdt)
{
	size_t len;
	aml_node_t *ast;
	aml_block_header_t hdr;

	if(!dsdt || !checksum_check(dsdt, dsdt->header.length))
		return;
	len = dsdt->header.length - sizeof(dsdt->header);
	if(!(ast = aml_parse(dsdt->src, len)))
		PANIC("Failed to parse AML code!\n", 0);
	ast_print(ast);
	print_memory(dsdt->src, len); // TODO rm
	aml_get_header(ast, &hdr);
	if(!aml_check_header(&hdr))
	{
		// TODO Panic
		printf("invalid AML header\n");
	}
	exec_aml(ast);
}
