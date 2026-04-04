/*
 * Copyright 2026 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

use flate2::read::GzDecoder;
use std::{env, fs, fs::File, io, io::BufRead, path::PathBuf, slice};

fn parse_hex_u16(s: &str) -> Option<u16> {
	if s.len() > 4 {
		return None;
	}
	let mut val = 0;
	let mut mul = 1;
	for c in s.bytes().rev() {
		let c = match c {
			b'0'..=b'9' => c - b'0',
			b'A'..=b'F' => c - b'A' + 10,
			b'a'..=b'f' => c - b'a' + 10,
			_ => return None,
		};
		val += mul * c as u16;
		mul = mul.wrapping_mul(16);
	}
	Some(val)
}

fn parse_hex_u64(s: &str) -> Option<u64> {
	let mut val = 0;
	let mut mul = 1;
	for c in s.bytes().rev() {
		let c = match c {
			b'0'..=b'9' => c - b'0',
			b'A'..=b'F' => c - b'A' + 10,
			b'a'..=b'f' => c - b'a' + 10,
			_ => return None,
		};
		val += mul * c as u64;
		mul = mul.wrapping_mul(16);
	}
	Some(val)
}

/// If `font` is a URL, downloads it and returns the local path.
///
/// Otherwise, returns the path as-is.
fn resolve_font(font: &str) -> io::Result<PathBuf> {
	if !font.starts_with("https://") {
		return Ok(PathBuf::from(font));
	}
	let out_dir = env::var_os("OUT_DIR").expect("OUT_DIR environment variable not set");
	let dest = PathBuf::from(out_dir).join("font.hex");
	if !dest.exists() {
		let response = ureq::get(font).call().map_err(io::Error::other)?;
		let mut body = response.into_body();
		let mut file = File::create(&dest)?;
		io::copy(&mut body.as_reader(), &mut file)?;
	}
	Ok(dest)
}

/// Turns the font in a version usable in the kernel.
pub fn build(font: &str) -> io::Result<()> {
	let gz = font.ends_with(".gz");
	let font_path = resolve_font(font)?;
	let file = File::open(&font_path)?;
	let reader: Box<dyn io::Read> = if gz {
		Box::new(GzDecoder::new(file))
	} else {
		Box::new(file)
	};
	let reader = io::BufReader::new(reader);
	// Parse
	let mut font = vec![[0u64; 2]; u16::MAX as usize];
	for line in reader.lines() {
		let line = line?;
		let colon_off = line.find(":").unwrap();
		let (key, val) = line.split_at(colon_off);
		let val = &val[1..];
		let Some(key) = parse_hex_u16(key) else {
			continue;
		};
		match val.len() {
			32 => {
				let (a, b) = val.split_at(16);
				let (Some(a), Some(b)) = (parse_hex_u64(a), parse_hex_u64(b)) else {
					continue;
				};
				font[key as usize] = [a.to_be(), b.to_be()];
			}
			64 => {
				// TODO
			}
			_ => continue,
		}
	}
	// Write font file
	let out_dir = env::var_os("OUT_DIR").expect("OUT_DIR environment variable not set");
	let font_path = PathBuf::from(out_dir).join("font.bin");
	let bytes = font.as_flattened();
	let bytes = unsafe { slice::from_raw_parts(bytes.as_ptr() as *const u8, bytes.len() * 8) };
	fs::write(&font_path, bytes)?;
	Ok(())
}
