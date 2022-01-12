// Implementation of the ChaCha20 algorithm according to RFC 8439.

#include <stddef.h>
#include <stdint.h>

/// Constants used in the algorithm's state.
static const uint32_t CONSTANTS[4] = {
	0x61707865, 0x3320646e, 0x79622d32, 0x6b206574
};

// Performs the quarter round operation on `a`, `b`, `c` and `d`.
static inline void quarter_round(uint32_t *a, uint32_t *b, uint32_t *c,
	uint32_t *d)
{
	*a += *b;
	*d ^= *a;
	*d = (*d << 16) | (*d >> (32 - 16));

	*c += *d;
	*b ^= *c;
	*b = (*b << 12) | (*b >> (32 - 12));

	*a += *b;
	*d ^= *a;
	*d = (*d << 8) | (*d >> (32 - 8));

	*c += *d;
	*b ^= *c;
	*b = (*b << 7) | (*b >> (32 - 7));
}

// Generates a ChaCha20 block.
// TODO doc
static void get_block(uint32_t *k, uint32_t b, uint32_t *n, uint32_t *out)
{
	const uint32_t init_s[16] = {
		CONSTANTS[0], CONSTANTS[1], CONSTANTS[2], CONSTANTS[3],
		k[0], k[1], k[2], k[3],
		k[4], k[5], k[6], k[7],
		b, n[0], n[1], n[2]
	};

	for (size_t i = 0; i < 16; ++i)
		out[i] = init_s[i];
	for (size_t i = 0; i < 10; ++i)
	{
		quarter_round(&out[0], &out[4], &out[8],  &out[12]);
		quarter_round(&out[1], &out[5], &out[9],  &out[13]);
		quarter_round(&out[2], &out[6], &out[10], &out[14]);
		quarter_round(&out[3], &out[7], &out[11], &out[15]);
		quarter_round(&out[0], &out[5], &out[10], &out[15]);
		quarter_round(&out[1], &out[6], &out[11], &out[12]);
		quarter_round(&out[2], &out[7], &out[8],  &out[13]);
		quarter_round(&out[3], &out[4], &out[9],  &out[14]);
	}
	for (size_t i = 0; i < 16; ++i)
		out[i] += init_s[i];
}

// Encodes the given data in `buff` using ChaCha20, with the given key `k` and
// the given nonces `n`.
// `len` is the length of the buffer in bytes.
// `k` and `n` must have 8 and 3 elements respectively.
// It is important that nonces are not repeated for the same key.
// `out` is the buffer which will contain the result. Its length must be
// `ceil(len / 64) * 64`.
void chacha20_encode(uint8_t *buff, size_t len, uint32_t *k, uint32_t *n,
	uint8_t *out)
{
	for (size_t i = 0; i < len / 64; ++i)
	{
		uint8_t key_stream[64];
		get_block(k, i, n, (uint32_t *) &key_stream);

		for (size_t j = 0; j < 64; ++j)
			out[i * 64 + j] = buff[i * 64 + j] ^ key_stream[j];
	}

	if (len % 64 != 0)
	{
		const size_t i = len / 64;

		uint8_t key_stream[64];
		get_block(k, i, n, (uint32_t *) &key_stream);

		for (size_t j = 0; j < (len - (i * 64)); ++j)
			out[i * 64 + j] = buff[i * 64 + j] ^ key_stream[j];
	}
}
