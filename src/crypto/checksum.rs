//! This module implements checksum algorithms. A checksum is a value allowing to verify the
//! integrity of a structure.

/// Computes a checksum on `data` according to RFC1071.
pub fn compute_rfc1071(data: &[u8]) -> u32 {
	let mut sum: u32 = 0;
	let mut i = 0;

	// Main loop
	while i < (data.len() & !1) {
		sum += ((data[i + 1] as u32) << 8) | (data[i] as u32);
		i += 2;
	}

	// Add remaining byte
	if i < data.len() {
		sum += data[i] as u32;
	}

	// Folding 32-bits value into 16-bits
	while (sum >> 16) != 0 {
		sum = (sum & 0xffff) + (sum >> 16);
	}

	!sum
}

/// TODO doc
pub fn compute_crc32(_data: &[u8], _polynom: u32) {
	// TODO
	todo!();
}

#[cfg(test)]
mod test {
	#[test_suite]
	fn rfc1071_0() {
		for i in 0..=u8::MAX {
			assert_eq!(compute_rfc1071(&[i]), !i);
		}
	}

	// TODO More tests on RFC1071

	// TODO Test CRC32
}
