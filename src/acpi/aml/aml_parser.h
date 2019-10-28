#ifndef AML_PARSER_H
# define AML_PARSER_H

# include <memory/memory.h>
# include <libc/errno.h>

# include <debug/debug.h> // TODO rm
# include <libc/stdio.h> // TODO rm

# define ZERO_OP				((char) 0x00)
# define ONE_OP					((char) 0x01)
# define ALIAS_OP				((char) 0x06)
# define NAME_OP				((char) 0x08)
# define SCOPE_OP				((char) 0x10)
# define BUFFER_OP				((char) 0x11)
# define PACKAGE_OP				((char) 0x12)
# define VAR_PACKAGE_OP			((char) 0x13)
# define COND_REF_OF_OP			((char) 0x12)
# define LOAD_TABLE_OP			((char) 0x1f)
# define ACQUIRE_OP				((char) 0x23)
# define WAIT_OP				((char) 0x25)
# define FROM_BCD_OP			((char) 0x28)
# define TO_BCD_OP				((char) 0x29)
# define REVISION_OP			((char) 0x30)
# define TIMER_OP				((char) 0x33)
# define STORE_OP				((char) 0x70)
# define REF_OF_OP				((char) 0x71)
# define ADD_OP					((char) 0x72)
# define CONCAT_OP				((char) 0x73)
# define SUBTRACT_OP			((char) 0x74)
# define INCREMENT_OP			((char) 0x75)
# define DECREMENT_OP			((char) 0x76)
# define MULTIPLY_OP			((char) 0x77)
# define DIVIDE_OP				((char) 0x78)
# define SHIFT_LEFT_OP			((char) 0x79)
# define SHIFT_RIGHT_OP			((char) 0x7a)
# define AND_OP					((char) 0x7b)
# define N_AND_OP				((char) 0x7c)
# define N_OR_OP				((char) 0x7e)
# define OR_OP					((char) 0x7d)
# define XOR_OP					((char) 0x7f)
# define NOT_OP					((char) 0x80)
# define FIND_SET_LEFT_BIT_OP	((char) 0x81)
# define FIND_SET_RIGHT_BIT_OP	((char) 0x82)
# define DEREF_OF_OP			((char) 0x83)
# define CONCAT_RES_OP			((char) 0x84)
# define MOD_OP					((char) 0x85)
# define SIZE_OF_OP				((char) 0x87)
# define BANK_FIELD_OP			((char) 0x87)
# define INDEX_OP				((char) 0x88)
# define MATCH_OP				((char) 0x89)
# define OBJECT_TYPE_OP			((char) 0x8e)
# define L_AND_OP				((char) 0x90)
# define L_OR_OP				((char) 0x91)
# define L_NOT_OP				((char) 0x92)
# define L_EQUAL_OP				((char) 0x93)
# define L_GREATER_OP			((char) 0x94)
# define L_LESS_OP				((char) 0x95)
# define TO_BUFFER_OP			((char) 0x96)
# define TO_DECIMAL_STRING_OP	((char) 0x97)
# define TO_HEX_STRING_OP		((char) 0x98)
# define TO_INTEGER_OP			((char) 0x99)
# define TO_STRING_OP			((char) 0x9c)
# define COPY_OBJECT_OP			((char) 0x9d)
# define MID_OP					((char) 0x9e)
# define CONTINUE_OP			((char) 0x9f)
# define IF_OP					((char) 0xa0)
# define NOOP_OP				((char) 0xa3)
# define BREAK_OP				((char) 0xa5)
# define BREAKPOINT_OP			((char) 0xcc)
# define ONES_OP				((char) 0xff)

# define EXT_OP_PREFIX			((char) 0x5b)
# define OP_REGION_OP			((char) 0x80)

# define ARG0_OP				((char) 0x68)
# define ARG6_OP				((char) 0x6e)
# define LOCAL0_OP				((char) 0x60)
# define LOCAL7_OP				((char) 0x67)

# define DUAL_NAME_PREFIX		((char) 0x2e)
# define MULTI_NAME_PREFIX		((char) 0x2f)

# define BYTE_PREFIX			((char) 0x0a)
# define WORD_PREFIX			((char) 0x0b)
# define DWORD_PREFIX			((char) 0x0c)
# define QWORD_PREFIX			((char) 0x0e)
# define STRING_PREFIX			((char) 0x0d)

# define IS_LEAD_NAME_CHAR(c)	(((c) >= 'A' && (c) <= 'Z') || (c) == '_')
# define IS_DIGIT_CHAR(c)		((c) >= '0' && (c) <= '9')
# define IS_NAME_CHAR(c)		(IS_LEAD_NAME_CHAR(c) || IS_DIGIT_CHAR(c))
# define IS_ROOT_CHAR(c)		((c) == '\\')
# define IS_PREFIX_CHAR(c)		((c) == '^')
# define IS_ARG_OP(c)			((c) >= ARG0_OP && (c) <= ARG6_OP)
# define IS_LOCAL_OP(c)			((c) >= LOCAL0_OP && (c) <= LOCAL7_OP)

enum node_type
{
	AML_CODE,
	AML_DEF_BLOCK_HEADER,
	AML_TABLE_SIGNATURE,
	AML_TABLE_LENGTH,
	AML_SPEC_COMPLIANCE,
	AML_CHECK_SUM,
	AML_OEM_ID,
	AML_OEM_TABLE_ID,
	AML_OEM_REVISION,
	AML_CREATOR_ID,
	AML_CREATOR_REVISION,
	AML_ROOT_CHAR,
	AML_NAME_SEG,
	AML_NAME_STRING,
	AML_PREFIX_PATH,
	AML_NAME_PATH,
	AML_DUAL_NAME_PATH,
	AML_MULTI_NAME_PATH,
	AML_SEG_COUNT,
	AML_SIMPLE_NAME,
	AML_SUPER_NAME,
	AML_NULL_NAME,
	AML_TARGET,
	AML_COMPUTATIONAL_DATA,
	AML_DATA_OBJECT,
	AML_DATA_REF_OBJECT,
	AML_BYTE_CONST,
	AML_BYTE_PREFIX,
	AML_WORD_CONST,
	AML_WORD_PREFIX,
	AML_DWORD_CONST,
	AML_DWORD_PREFIX,
	AML_QWORD_CONST,
	AML_QWORD_PREFIX,
	AML_STRING,
	AML_STRING_PREFIX,
	AML_CONST_OBJ,
	AML_BYTE_LIST,
	AML_BYTE_DATA,
	AML_WORD_DATA,
	AML_DWORD_DATA,
	AML_QWORD_DATA,
	AML_ASCII_CHAR_LIST,
	AML_ASCII_CHAR,
	AML_NULL_CHAR,
	AML_ZERO_OP,
	AML_ONE_OP,
	AML_ONES_OP,
	AML_REVISION_OP,
	AML_PKG_LENGTH,
	AML_PKG_LEAD_BYTE,
	AML_OBJECT,
	AML_TERM_OBJ,
	AML_TERM_LIST,
	AML_TERM_ARG,
	AML_METHOD_INVOCATION,
	AML_TERM_ARG_LIST,
	AML_NAME_SPACE_MODIFIER_OBJ,
	AML_DEF_ALIAS,
	AML_DEF_NAME,
	AML_DEF_SCOPE,
	AML_NAMED_OBJ,
	AML_DEF_BANK_FIELD,
	AML_BANK_VALUE,
	AML_FIELD_FLAGS,
	AML_FIELD_LIST,
	AML_NAMED_FIELD,
	AML_RESERVED_FIELD,
	AML_ACCESS_FIELD,
	AML_ACCESS_TYPE,
	AML_ACCESS_ATTRIB,
	AML_CONNECT_FIELD,
	AML_DEF_CREATE_BIT_FIELD,
	AML_CREATE_BIT_FIELD_OP,
	AML_SOURCE_BUFF,
	AML_BIT_INDEX,
	AML_DEF_CREATE_BYTE_FIELD,
	AML_CREATE_BYTE_FIELD_OP,
	AML_BYTE_INDEX,
	AML_DEF_CREATE_D_WORD_FIELD,
	AML_CREATE_D_WORD_FIELD_OP,
	AML_DEF_CREATE_FIELD,
	AML_CREATE_FIELD_OP,
	AML_NUM_BITS,
	AML_DEF_CREATE_Q_WORD_FIELD,
	AML_CREATE_Q_WORD_FIELD_OP,
	AML_DEF_CREATE_WORD_FIELD,
	AML_CREATE_WORD_FIELD_OP,
	AML_DEF_DATA_REGION,
	AML_DATA_REGION_OP,
	AML_DEF_DEVICE,
	AML_DEVICE_OP,
	AML_DEF_EVENT,
	AML_EVENT_OP,
	AML_DEF_EXTERNAL,
	AML_EXTERNAL_OP,
	AML_OBJECT_TYPE,
	AML_ARGUMENT_COUNT,
	AML_DEF_FIELD,
	AML_FIELD_OP,
	AML_DEF_INDEX_FIELD,
	AML_INDEX_FIELD_OP,
	AML_DEF_METHOD,
	AML_METHOD_OP,
	AML_METHOD_FLAGS,
	AML_DEF_MUTEX,
	AML_MUTEX_OP,
	AML_SYNC_FLAGS,
	AML_DEF_OP_REGION,
	AML_OP_REGION_OP,
	AML_REGION_SPACE,
	AML_REGION_OFFSET,
	AML_REGION_LEN,
	AML_DEF_POWER_RES,
	AML_POWER_RES_OP,
	AML_SYSTEM_LEVEL,
	AML_RESOURCE_ORDER,
	AML_DEF_PROCESSOR,
	AML_PROCESSOR_OP,
	AML_PROC_ID,
	AML_PBLK_ADDR,
	AML_PBLK_LEN,
	AML_DEF_THERMAL_ZONE,
	AML_THERMAL_ZONE_OP,
	AML_EXTENDED_ACCESS_FIELD,
	AML_EXTENDED_ACCESS_ATTRIB,
	AML_FIELD_ELEMENT,
	AML_TYPE1_OPCODE,
	AML_DEF_BREAK,
	AML_DEF_BREAK_POINT,
	AML_DEF_CONTINUE,
	AML_DEF_ELSE,
	AML_DEF_FATAL,
	AML_FATAL_OP,
	AML_FATAL_TYPE,
	AML_FATAL_CODE,
	AML_FATAL_ARG,
	AML_DEF_IF_ELSE,
	AML_PREDICATE,
	AML_DEF_LOAD,
	AML_LOAD_OP,
	AML_DDB_HANDLE_OBJECT,
	AML_DEF_NOOP,
	AML_DEF_NOTIFY,
	AML_NOTIFY_OP,
	AML_NOTIFY_OBJECT,
	AML_NOTIFY_VALUE,
	AML_DEF_RELEASE,
	AML_RELEASE_OP,
	AML_MUTEX_OBJECT,
	AML_DEF_RESET,
	AML_RESET_OP,
	AML_EVENT_OBJECT,
	AML_DEF_RETURN,
	AML_RETURN_OP,
	AML_ARG_OBJECT,
	AML_DEF_SIGNAL,
	AML_SIGNAL_OP,
	AML_DEF_SLEEP,
	AML_SLEEP_OP,
	AML_MSEC_TIME,
	AML_DEF_STALL,
	AML_STALL_OP,
	AML_USEC_TIME,
	AML_DEF_WHILE,
	AML_WHILE_OP,
	AML_TYPE2_OPCODE,
	AML_TYPE6_OPCODE,
	AML_DEF_ACQUIRE,
	AML_ACQUIRE_OP,
	AML_TIMEOUT,
	AML_DEF_ADD,
	AML_ADD_OP,
	AML_OPERAND,
	AML_DEF_AND,
	AML_AND_OP,
	AML_DEF_BUFFER,
	AML_BUFFER_OP,
	AML_BUFFER_SIZE,
	AML_DEF_CONCAT,
	AML_CONCAT_OP,
	AML_DATA,
	AML_DEF_CONCAT_RES,
	AML_CONCAT_RES_OP,
	AML_BUF_DATA,
	AML_DEF_COND_REF_OF,
	AML_COND_REF_OF_OP,
	AML_DEF_COPY_OBJECT,
	AML_COPY_OBJECT_OP,
	AML_DEF_DECREMENT,
	AML_DECREMENT_OP,
	AML_DEF_DEREF_OF,
	AML_DEREF_OF_OP,
	AML_OBJ_REFERENCE,
	AML_DEF_DIVIDE,
	AML_DIVIDE_OP,
	AML_DIVIDEND,
	AML_DIVISOR,
	AML_REMAINDER,
	AML_QUOTIENT,
	AML_DEF_FIND_SET_LEFT_BIT,
	AML_FIND_SET_LEFT_BIT_OP,
	AML_DEF_FIND_SET_RIGHT_BIT,
	AML_FIND_SET_RIGHT_BIT_OP,
	AML_DEF_FROM_BCD,
	AML_FROM_BCD_OP,
	AML_BCD_VALUE,
	AML_DEF_INCREMENT,
	AML_INCREMENT_OP,
	AML_DEF_INDEX,
	AML_INDEX_OP,
	AML_BUFF_PKG_STR_OBJ,
	AML_INDEX_VALUE,
	AML_DEF_L_AND,
	AML_LAND_OP,
	AML_DEF_L_EQUAL,
	AML_LEQUAL_OP,
	AML_DEF_L_GREATER,
	AML_LGREATER_OP,
	AML_DEF_L_GREATER_EQUAL,
	AML_LGREATER_EQUAL_OP,
	AML_DEF_L_LESS,
	AML_LLESS_OP,
	AML_DEF_L_LESS_EQUAL,
	AML_LLESS_EQUAL_OP,
	AML_DEF_L_NOT,
	AML_LNOT_OP,
	AML_DEF_L_NOT_EQUAL,
	AML_LNOT_EQUAL_OP,
	AML_DEF_LOAD_TABLE,
	AML_LOAD_TABLE_OP,
	AML_DEF_L_OR,
	AML_LOR_OP,
	AML_DEF_MATCH,
	AML_MATCH_OP,
	AML_SEARCH_PKG,
	AML_MATCH_OPCODE,
	AML_START_INDEX,
	AML_DEF_MID,
	AML_MID_OP,
	AML_MID_OBJ,
	AML_DEF_MOD,
	AML_MOD_OP,
	AML_DEF_MULTIPLY,
	AML_MULTIPLY_OP,
	AML_DEF_N_AND,
	AML_NAND_OP,
	AML_DEF_N_OR,
	AML_NOR_OP,
	AML_DEF_NOT,
	AML_NOT_OP,
	AML_DEF_OBJECT_TYPE,
	AML_OBJECT_TYPE_OP,
	AML_DEF_OR,
	AML_OR_OP,
	AML_DEF_PACKAGE,
	AML_PACKAGE_OP,
	AML_DEF_VAR_PACKAGE,
	AML_VAR_PACKAGE_OP,
	AML_NUM_ELEMENTS,
	AML_VAR_NUM_ELEMENTS,
	AML_PACKAGE_ELEMENT_LIST,
	AML_PACKAGE_ELEMENT,
	AML_DEF_REF_OF,
	AML_REF_OF_OP,
	AML_DEF_SHIFT_LEFT,
	AML_SHIFT_LEFT_OP,
	AML_SHIFT_COUNT,
	AML_DEF_SHIFT_RIGHT,
	AML_SHIFT_RIGHT_OP,
	AML_DEF_SIZE_OF,
	AML_SIZE_OF_OP,
	AML_DEF_STORE,
	AML_STORE_OP,
	AML_DEF_SUBTRACT,
	AML_SUBTRACT_OP,
	AML_DEF_TIMER,
	AML_TIMER_OP,
	AML_DEF_TO_BCD,
	AML_TO_BCD_OP,
	AML_DEF_TO_BUFFER,
	AML_TO_BUFFER_OP,
	AML_DEF_TO_DECIMAL_STRING,
	AML_TO_DECIMAL_STRING_OP,
	AML_DEF_TO_HEX_STRING,
	AML_TO_HEX_STRING_OP,
	AML_DEF_TO_INTEGER,
	AML_TO_INTEGER_OP,
	AML_DEF_TO_STRING,
	AML_LENGTH_ARG,
	AML_TO_STRING_OP,
	AML_DEF_WAIT,
	AML_WAIT_OP,
	AML_DEF_X_OR,
	AML_XOR_OP,
	AML_ARG_OBJ,
	AML_ARG0_OP,
	AML_ARG1_OP,
	AML_ARG2_OP,
	AML_ARG3_OP,
	AML_ARG4_OP,
	AML_ARG5_OP,
	AML_ARG6_OP,
	AML_LOCAL_OBJ,
	AML_LOCAL0_OP,
	AML_LOCAL1_OP,
	AML_LOCAL2_OP,
	AML_LOCAL3_OP,
	AML_LOCAL4_OP,
	AML_LOCAL5_OP,
	AML_LOCAL6_OP,
	AML_LOCAL7_OP,
	AML_DEBUG_OBJ,
	AML_DEBUG_OP
};

typedef struct aml_node
{
	struct aml_node *children;
	struct aml_node *next;

	enum node_type type;
	const void *ptr;

	const char *data;
	size_t data_length;
} aml_node_t;

typedef aml_node_t *(*parse_func_t)(const char **, size_t *);

aml_node_t *parse_node(enum node_type type, const char **src, size_t *len,
	size_t n, ...);
aml_node_t *parse_serie(const char **src, size_t *len, size_t n, ...);
aml_node_t *parse_list(enum node_type type, const char **src, size_t *len,
	parse_func_t f);
aml_node_t *parse_string(const char **src, size_t *len,
	size_t str_len, parse_func_t f);
aml_node_t *parse_either(enum node_type type, const char **src,
	size_t *len, size_t n, ...);

aml_node_t *node_new(enum node_type type, const char *data, size_t length);
void node_add_child(aml_node_t *node, aml_node_t *child);
void ast_print(const aml_node_t *ast);
void node_free(aml_node_t *node);
void ast_free(aml_node_t *ast);

uint8_t aml_get_byte(aml_node_t *node);
uint16_t aml_get_word(aml_node_t *node);
uint32_t aml_get_dword(aml_node_t *node);

aml_node_t *data_object(const char **src, size_t *len);
aml_node_t *byte_data(const char **src, size_t *len);
aml_node_t *word_data(const char **src, size_t *len);
aml_node_t *dword_data(const char **src, size_t *len);
aml_node_t *qword_data(const char **src, size_t *len);

aml_node_t *string(const char **src, size_t *len);

aml_node_t *name_seg(const char **src, size_t *len);
aml_node_t *name_string(const char **src, size_t *len);

aml_node_t *access_type(const char **src, size_t *len);
aml_node_t *access_attrib(const char **src, size_t *len);
aml_node_t *extended_access_attrib(const char **src, size_t *len);
aml_node_t *access_length(const char **src, size_t *len);

aml_node_t *pkg_length(const char **src, size_t *len);

aml_node_t *namespace_modifier_obj(const char **src, size_t *len);

aml_node_t *def_bank_field(const char **src, size_t *len);
aml_node_t *bank_value(const char **src, size_t *len);

aml_node_t *field_flags(const char **src, size_t *len);
aml_node_t *field_list(const char **src, size_t *len);

aml_node_t *named_obj(const char **src, size_t *len);
aml_node_t *def_op_region(const char **src, size_t *len);

aml_node_t *data_ref_object(const char **src, size_t *len);

aml_node_t *def_break(const char **src, size_t *len);
aml_node_t *def_breakpoint(const char **src, size_t *len);
aml_node_t *def_continue(const char **src, size_t *len);
aml_node_t *def_else(const char **src, size_t *len);
aml_node_t *def_fatal(const char **src, size_t *len);
aml_node_t *def_ifelse(const char **src, size_t *len);
aml_node_t *predicate(const char **src, size_t *len);
aml_node_t *def_load(const char **src, size_t *len);
aml_node_t *def_noop(const char **src, size_t *len);
aml_node_t *def_notify(const char **src, size_t *len);
aml_node_t *def_release(const char **src, size_t *len);
aml_node_t *def_reset(const char **src, size_t *len);
aml_node_t *def_return(const char **src, size_t *len);
aml_node_t *def_signal(const char **src, size_t *len);
aml_node_t *def_sleep(const char **src, size_t *len);
aml_node_t *def_stall(const char **src, size_t *len);
aml_node_t *def_while(const char **src, size_t *len);

aml_node_t *def_buffer(const char **src, size_t *len);
aml_node_t *def_package(const char **src, size_t *len);
aml_node_t *def_var_package(const char **src, size_t *len);

aml_node_t *type1_opcode(const char **src, size_t *len);
aml_node_t *type2_opcode(const char **src, size_t *len);

aml_node_t *arg_obj(const char **src, size_t *len);
aml_node_t *local_obj(const char **src, size_t *len);

aml_node_t *term_list(const char **src, size_t *len);
aml_node_t *term_arg(const char **src, size_t *len);

aml_node_t *aml_parse(const char *src, const size_t len);

#endif
