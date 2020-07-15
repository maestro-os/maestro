#ifndef SELFTEST_H
# define SELFTEST_H

# define ASSERT(x)\
	do\
	{\
		printf("%c", (x ? '.' : 'F'));\
		return;\
	}\
	while(0)

typedef void (*test_suite_func_t)(void);

void test_bitfield(void);
void test_avl(void);
void test_buddy(void);

void test_buddy_duplicates(void);
void test_kmalloc_bulk(void);

void run_selftest(void);

#endif
