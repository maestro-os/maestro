#ifndef AML_H
# define AML_H

# include <kernel.h>
# include <util/attr.h>

__ATTR_PACKED
struct aml_block_header
{
	uint32_t table_signature;
	uint32_t table_length;
	uint8_t spec_compliance;
	uint8_t checksum;
	uint8_t OEM_id[6];
	uint8_t OEM_tableid[8];
	uint32_t OEM_revision;
	uint32_t creator_id;
	uint32_t creator_revision;
};

typedef struct aml_block_header aml_block_header_t;

#endif
