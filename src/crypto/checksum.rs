//! This module implements checksum algorithms. A checksum is a value allowing
//! to verify the integrity of a structure.

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
///
/// Arguments:
/// - `table` is filled with the table's values.
/// - `polynom` is the polynom.
pub fn compute_crc32_lookuptable(table: &mut [u32; 256], polynom: u32) {
	// Little endian
	let mut i = table.len() / 2;
	let mut crc = 1;

	while i > 0 {
		if crc & 1 != 0 {
			crc = (crc >> 1) ^ polynom;
		} else {
			crc = crc >> 1;
		}

		for j in (0..table.len()).step_by(2 * i) {
			table[i ^ j] = crc ^ table[j];
		}

		i = i >> 1;
	}
}

/// Computes the CRC32 checksum on the given data `data` with the given table
/// `table` for the wanted generator polynomial.
pub fn compute_crc32(data: &[u8], table: &[u32; 256]) -> u32 {
	// Sarwate algorithm
	let mut crc = !0u32;

	for b in data {
		let i = ((crc as usize) ^ (*b as usize)) & 0xff;
		crc = table[i] ^ (crc >> 8);
	}

	!crc
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn rfc1071_0() {
		for i in 0..=u16::MAX {
			let data = [(i & 0xff) as _, (i >> 8) as _];
			assert_eq!(compute_rfc1071(&data), !(i as u32));
		}
	}

	// TODO More tests on RFC1071

	// TODO
	/*#[test_case]
	fn crc32_0() {
		for polynom in 0..u8::MAX {
			let mut lookup_table = [0; 256];
			compute_crc32_lookuptable(&mut lookup_table, polynom as _);
			crate::println!("table: {} {:?}", polynom, lookup_table); // TODO rm

			for i in 0..=u16::MAX {
				let data = [
					i as _,
					(i >> 8) as _,
					0,
					0,
					0,
					0,
				];
				let checksum = compute_crc32(&data, &lookup_table);

				let check = [
					data[0],
					data[1],
					checksum as _,
					(checksum >> 8) as _,
					(checksum >> 16) as _,
					(checksum >> 24) as _,
				];
				assert_eq!(compute_crc32(&check, &lookup_table), 0);
			}
		}
	}*/

	// TODO More tests on CRC32
}
