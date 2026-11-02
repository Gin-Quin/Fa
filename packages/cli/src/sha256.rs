//! SHA-256 (FIPS 180-4), for the hashes of the lockfile (advanced brief
//! §17.3, decision D45): small enough to keep the command free of a crypto
//! dependency, and only used on the sources of dependencies.

/// The round constants.
const K: [u32; 64] = [
	0x428a_2f98,
	0x7137_4491,
	0xb5c0_fbcf,
	0xe9b5_dba5,
	0x3956_c25b,
	0x59f1_11f1,
	0x923f_82a4,
	0xab1c_5ed5,
	0xd807_aa98,
	0x1283_5b01,
	0x2431_85be,
	0x550c_7dc3,
	0x72be_5d74,
	0x80de_b1fe,
	0x9bdc_06a7,
	0xc19b_f174,
	0xe49b_69c1,
	0xefbe_4786,
	0x0fc1_9dc6,
	0x240c_a1cc,
	0x2de9_2c6f,
	0x4a74_84aa,
	0x5cb0_a9dc,
	0x76f9_88da,
	0x983e_5152,
	0xa831_c66d,
	0xb003_27c8,
	0xbf59_7fc7,
	0xc6e0_0bf3,
	0xd5a7_9147,
	0x06ca_6351,
	0x1429_2967,
	0x27b7_0a85,
	0x2e1b_2138,
	0x4d2c_6dfc,
	0x5338_0d13,
	0x650a_7354,
	0x766a_0abb,
	0x81c2_c92e,
	0x9272_2c85,
	0xa2bf_e8a1,
	0xa81a_664b,
	0xc24b_8b70,
	0xc76c_51a3,
	0xd192_e819,
	0xd699_0624,
	0xf40e_3585,
	0x106a_a070,
	0x19a4_c116,
	0x1e37_6c08,
	0x2748_774c,
	0x34b0_bcb5,
	0x391c_0cb3,
	0x4ed8_aa4a,
	0x5b9c_ca4f,
	0x682e_6ff3,
	0x748f_82ee,
	0x78a5_636f,
	0x84c8_7814,
	0x8cc7_0208,
	0x90be_fffa,
	0xa450_6ceb,
	0xbef9_a3f7,
	0xc671_78f2,
];

/// An incremental SHA-256.
#[derive(Clone)]
pub(crate) struct Sha256 {
	state: [u32; 8],
	block: [u8; 64],
	filled: usize,
	length: u64,
}

impl Sha256 {
	/// A new hash.
	pub(crate) fn new() -> Self {
		Self {
			state: [
				0x6a09_e667,
				0xbb67_ae85,
				0x3c6e_f372,
				0xa54f_f53a,
				0x510e_527f,
				0x9b05_688c,
				0x1f83_d9ab,
				0x5be0_cd19,
			],
			block: [0; 64],
			filled: 0,
			length: 0,
		}
	}

	/// Adds bytes.
	pub(crate) fn update(&mut self, mut bytes: &[u8]) {
		self.length = self.length.wrapping_add(bytes.len() as u64);
		while !bytes.is_empty() {
			let take = (64 - self.filled).min(bytes.len());
			self.block[self.filled..self.filled + take].copy_from_slice(&bytes[..take]);
			self.filled += take;
			bytes = &bytes[take..];
			if self.filled == 64 {
				let block = self.block;
				self.compress(&block);
				self.filled = 0;
			}
		}
	}

	/// The digest.
	pub(crate) fn finish(mut self) -> [u8; 32] {
		let bits = self.length.wrapping_mul(8);
		let mut padding = vec![0x80u8];
		let after = (self.filled + 1) % 64;
		let zeros = if after <= 56 { 56 - after } else { 120 - after };
		padding.extend(std::iter::repeat_n(0, zeros));
		padding.extend_from_slice(&bits.to_be_bytes());
		let length = self.length;
		self.update(&padding);
		self.length = length;
		let mut out = [0; 32];
		for (index, word) in self.state.iter().enumerate() {
			out[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
		}
		out
	}

	/// Compresses one block.
	fn compress(&mut self, block: &[u8; 64]) {
		let mut w = [0u32; 64];
		for (index, chunk) in block.as_chunks::<4>().0.iter().enumerate() {
			w[index] = u32::from_be_bytes(*chunk);
		}
		for index in 16..64 {
			let s0 = w[index - 15].rotate_right(7)
				^ w[index - 15].rotate_right(18)
				^ (w[index - 15] >> 3);
			let s1 = w[index - 2].rotate_right(17)
				^ w[index - 2].rotate_right(19)
				^ (w[index - 2] >> 10);
			w[index] = w[index - 16]
				.wrapping_add(s0)
				.wrapping_add(w[index - 7])
				.wrapping_add(s1);
		}
		let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
		for index in 0..64 {
			let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
			let choice = (e & f) ^ (!e & g);
			let t1 = h
				.wrapping_add(s1)
				.wrapping_add(choice)
				.wrapping_add(K[index])
				.wrapping_add(w[index]);
			let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
			let majority = (a & b) ^ (a & c) ^ (b & c);
			let t2 = s0.wrapping_add(majority);
			h = g;
			g = f;
			f = e;
			e = d.wrapping_add(t1);
			d = c;
			c = b;
			b = a;
			a = t1.wrapping_add(t2);
		}
		for (state, value) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
			*state = state.wrapping_add(value);
		}
	}
}

/// The lowercase hexadecimal digest of bytes.
pub(crate) fn hex(bytes: &[u8]) -> String {
	let mut hash = Sha256::new();
	hash.update(bytes);
	to_hex(&hash.finish())
}

/// Bytes in lowercase hexadecimal.
pub(crate) fn to_hex(bytes: &[u8]) -> String {
	bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn known_digests() {
		assert_eq!(
			hex(b""),
			"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
		);
		assert_eq!(
			hex(b"abc"),
			"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
		);
		assert_eq!(
			hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
			"248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
		);
		let million = vec![b'a'; 1_000_000];
		assert_eq!(
			hex(&million),
			"cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
		);
		// incremental updates of odd sizes
		let mut hash = Sha256::new();
		for chunk in million.chunks(997) {
			hash.update(chunk);
		}
		assert_eq!(
			to_hex(&hash.finish()),
			"cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
		);
	}
}
