#include <selftest/selftest.h>
#include <libc/stdio.h>
#include <stdint.h>
#include <util/util.h>

static void test0(void)
{
	uint8_t bitfield = 0;

	bitfield_set(&bitfield, 0);
	ASSERT(bitfield == 0b10000000);
}

static void test1(void)
{
	uint8_t bitfield = 0;

	bitfield_set(&bitfield, 1);
	ASSERT(bitfield == 0b01000000);
}

static void test2(void)
{
	uint8_t bitfield[2];

	bzero(bitfield, sizeof(bitfield));
	bitfield_set(bitfield, 8);
	ASSERT(bitfield[0] == 0 && bitfield[1] == 0b10000000);
}

static void test3(void)
{
	uint8_t bitfield[2];

	bzero(bitfield, sizeof(bitfield));
	bitfield_set_range(bitfield, 0, 1);
	ASSERT(bitfield[0] == 0b10000000 && bitfield[1] == 0);
}

static void test4(void)
{
	uint8_t bitfield[2];

	bzero(bitfield, sizeof(bitfield));
	bitfield_set_range(bitfield, 0, 8);
	//printf("%i %i\n", bitfield[0], bitfield[1]);
	ASSERT(bitfield[0] == 0b11111111 && bitfield[1] == 0);
}

static void test5(void)
{
	uint8_t bitfield[2];

	bzero(bitfield, sizeof(bitfield));
	bitfield_set_range(bitfield, 0, 9);
	ASSERT(bitfield[0] == 0b11111111 && bitfield[1] == 0b10000000);
}

static void test6(void)
{
	uint8_t bitfield[2];

	bzero(bitfield, sizeof(bitfield));
	bitfield_set_range(bitfield, 1, 9);
	ASSERT(bitfield[0] == 0b01111111 && bitfield[1] == 0b10000000);
}

void test_bitfield(void)
{
	printf("%s: ", __func__);
	test0();
	test1();
	test2();
	test3();
	test4();
	test5();
	test6();
	printf("\n");
}
