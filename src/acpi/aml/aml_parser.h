#ifndef AML_PARSER_H
# define AML_PARSER_H

# include <memory/memory.h>
# include <libc/errno.h>

# define NEW_NODE()	(kmalloc_zero(sizeof(aml_node_t), 0))

typedef struct aml_node
{
	struct aml_node *children;
	struct aml_node *next;

	const char *data;
} aml_node_t;

typedef aml_node_t *(*parse_func_t)(const char **, size_t *);

aml_node_t *parse_node(const char **src, size_t *len, size_t n, ...);
aml_node_t *parse_serie(const char **src, size_t *len, size_t n, ...);
aml_node_t *parse_string(const char **src, size_t *len,
	size_t str_len, parse_func_t f);
aml_node_t *parse_either(const char **src, size_t *len, size_t n, ...);
void node_add_child(aml_node_t *node, aml_node_t *child);
void node_free(aml_node_t *node);
void ast_free(aml_node_t *ast);

aml_node_t *byte_data(const char **src, size_t *len);
aml_node_t *word_data(const char **src, size_t *len);
aml_node_t *dword_data(const char **src, size_t *len);
aml_node_t *qword_data(const char **src, size_t *len);

aml_node_t *def_block_header(const char **src, size_t *len);

aml_node_t *namespace_modifier_obj(const char **src, size_t *len);

aml_node_t *type1_opcode(const char **src, size_t *len);
aml_node_t *type2_opcode(const char **src, size_t *len);

aml_node_t *aml_parse(const char *src, const size_t len);

#endif
