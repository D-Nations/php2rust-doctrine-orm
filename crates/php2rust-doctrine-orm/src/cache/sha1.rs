//! `sha1()`: the SHA-1 digest of a byte string, in lowercase hex.
//!
//! Upstream: none

use std::fmt::Write as _;

/// `sha1($data)`.
#[allow(clippy::many_single_char_names, reason = "the names of FIPS 180-4")]
pub fn sha1(data: &[u8]) -> String {
    let mut h: [u32; 5] = [
        0x6745_2301,
        0xEFCD_AB89,
        0x98BA_DCFE,
        0x1032_5476,
        0xC3D2_E1F0,
    ];
    let mut message = data.to_vec();
    let bit_len = u64::try_from(data.len())
        .unwrap_or(u64::MAX)
        .wrapping_mul(8);
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in message.chunks_exact(64) {
        let mut w = [0_u32; 80];
        for (i, word) in chunk.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = h;
        for (i, word) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | (!b & d), 0x5A82_7999),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
                _ => (b ^ c ^ d, 0xCA62_C1D6),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }
    h.iter().fold(String::with_capacity(40), |mut hex, word| {
        let _ = write!(hex, "{word:08x}");
        hex
    })
}

#[cfg(test)]
mod tests {
    use super::sha1;

    #[test]
    fn digests() {
        assert_eq!("da39a3ee5e6b4b0d3255bfef95601890afd80709", sha1(b""));
        assert_eq!("a9993e364706816aba3e25717850c26c9cd0d89d", sha1(b"abc"));
        assert_eq!(
            "84983e441c3bd26ebaae4aa1f95129e5e54670f1",
            sha1(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq")
        );
    }
}
