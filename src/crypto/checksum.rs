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

/// Computes the lookup table for the given generator polynomial.
/// `table` is filled with the table's values.
/// `polynom` is the polynom.
fn compute_crc32_lookuptable(table: &mut [u32; 256], polynom: u32) {
	let mut i = table.len() / 2;
	let mut crc = 1;

	while i > 0 {
		if crc & 1 != 0 {
			crc = (crc >> 1) ^ polynom;
		} else {
			crc >>= 1;
		}

		for j in (0..table.len()).step_by(2 * i) {
			table[i + j] = crc ^ table[j];
		}

		i /= 2;
	}
}

/// Computes the CRC32 checksum on the given data `data` with the given generator polynomial
/// `polynom`.
pub fn compute_crc32(data: &[u8], polynom: u32) -> u32 {
	let mut lookup_table: [u32; 256] = [0; 256];
	compute_crc32_lookuptable(&mut lookup_table, polynom);

	let mut crc: u32 = !0;

	for b in data {
		let i = ((b ^ (crc as u8)) & 0xff) as usize;
		crc = lookup_table[i] ^ (crc >> 8);
	}

	!crc
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn rfc1071_0() {
		for i in 0..=u8::MAX {
			assert_eq!(compute_rfc1071(&[i]), !i as u32);
		}
	}

	// TODO More tests on RFC1071

	// TODO Test CRC32
}
