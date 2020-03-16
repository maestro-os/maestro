#ifndef SELFTEST_H
# define SELFTEST_H

# define ASSERT(x)	printf("%c", (x ? '.' : 'F'))

typedef void (*test_suite_func_t)(void);

void test_bitfield(void);
void test_buddy(void);

void run_selftest(void);

#endif
