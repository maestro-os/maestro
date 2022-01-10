//! Implementation of the ChaCha20 algorithm according to RFC 8439.

extern "C" {
	fn chacha20_encode(buff: *const u8, len: usize, k: *const u32, n: *const u32, out: *mut u8);
}

/// Encodes the given data in `buff` using ChaCha20, with the given key `k` and the given nonces
/// `n`.
/// It is important that nonces are not repeated for the same key.
/// `out` is the buffer which will contain the result. Its length must be
/// `ceil(buff.len() / 64) * 64`.
pub fn encode(buff: &[u8], k: &[u32; 8], n: &[u32; 3], out: &mut [u8]) {
	unsafe {
		chacha20_encode(buff.as_ptr(), buff.len(), k.as_ptr(), n.as_ptr(), out.as_mut_ptr());
	}
}

// TODO Use with C code
/*#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn quarter_round0() {
		let (a, b, c, d) = quarter_round(0x11111111, 0x01020304, 0x9b8d6f43, 0x01234567);

		assert_eq!(a, 0xea2a92f4);
		assert_eq!(b, 0xcb1cf8ce);
		assert_eq!(c, 0x4581472e);
		assert_eq!(d, 0x5881c4bb);
	}
}*/
