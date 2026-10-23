//! SHA-1, which names git's objects, and SHA-256, which names an LFS file's
//! contents. Both are small enough to keep here rather than depend on.

/// An object's name: the SHA-1 of its kind, length and contents.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Oid(pub [u8; 20]);

impl Oid {
    pub fn from_slice(bytes: &[u8]) -> Option<Oid> {
        bytes.try_into().ok().map(Oid)
    }

    pub fn from_hex(hex: &[u8]) -> Option<Oid> {
        if hex.len() != 40 {
            return None;
        }
        let mut out = [0; 20];
        for (byte, pair) in out.iter_mut().zip(hex.chunks(2)) {
            *byte = (nibble(pair[0])? << 4) | nibble(pair[1])?;
        }
        Some(Oid(out))
    }

    pub fn hex(&self) -> String {
        to_hex(&self.0)
    }

    /// The name git gives these contents as an object of this kind.
    pub fn of(kind: &str, contents: &[u8]) -> Oid {
        let mut sha = Sha1::new();
        sha.update(kind.as_bytes());
        sha.update(b" ");
        sha.update(contents.len().to_string().as_bytes());
        sha.update(&[0]);
        sha.update(contents);
        Oid(sha.finish())
    }
}

impl std::fmt::Debug for Oid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.hex())
    }
}

fn nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

pub fn to_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[usize::from(byte >> 4)] as char);
        out.push(DIGITS[usize::from(byte & 15)] as char);
    }
    out
}

/// What both hashes share: a 64-byte block buffer and a running length.
struct Blocks {
    buffer: [u8; 64],
    used: usize,
    length: u64,
}

impl Blocks {
    fn new() -> Blocks {
        Blocks {
            buffer: [0; 64],
            used: 0,
            length: 0,
        }
    }

    fn update(&mut self, mut data: &[u8], mut compress: impl FnMut(&[u8; 64])) {
        self.length += data.len() as u64;
        if self.used > 0 {
            let take = (64 - self.used).min(data.len());
            self.buffer[self.used..self.used + take].copy_from_slice(&data[..take]);
            self.used += take;
            data = &data[take..];
            if self.used < 64 {
                return;
            }
            compress(&self.buffer);
            self.used = 0;
        }
        let (chunks, rest) = data.as_chunks::<64>();
        for chunk in chunks {
            compress(chunk);
        }
        self.buffer[..rest.len()].copy_from_slice(rest);
        self.used = rest.len();
    }

    /// Pads the last block with the length in bits, big-endian, as both
    /// hashes do.
    fn finish(&mut self, mut compress: impl FnMut(&[u8; 64])) {
        let bits = self.length * 8;
        let mut tail = [0u8; 128];
        tail[..self.used].copy_from_slice(&self.buffer[..self.used]);
        tail[self.used] = 0x80;
        let total = if self.used < 56 { 64 } else { 128 };
        tail[total - 8..total].copy_from_slice(&bits.to_be_bytes());
        for block in tail[..total].as_chunks::<64>().0 {
            compress(block);
        }
    }
}

pub struct Sha1 {
    state: [u32; 5],
    blocks: Blocks,
}

impl Sha1 {
    pub fn new() -> Sha1 {
        Sha1 {
            state: [
                0x6745_2301,
                0xEFCD_AB89,
                0x98BA_DCFE,
                0x1032_5476,
                0xC3D2_E1F0,
            ],
            blocks: Blocks::new(),
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        let state = &mut self.state;
        self.blocks
            .update(data, |block| sha1_compress(state, block));
    }

    pub fn finish(mut self) -> [u8; 20] {
        let state = &mut self.state;
        self.blocks.finish(|block| sha1_compress(state, block));
        let mut out = [0; 20];
        for (chunk, word) in out.as_chunks_mut::<4>().0.iter_mut().zip(self.state) {
            chunk.copy_from_slice(&word.to_be_bytes());
        }
        out
    }
}

#[expect(
    clippy::many_single_char_names,
    reason = "named as FIPS 180-4 names them"
)]
fn sha1_compress(state: &mut [u32; 5], block: &[u8; 64]) {
    let mut w = [0u32; 80];
    for (i, word) in block.as_chunks::<4>().0.iter().enumerate() {
        w[i] = u32::from_be_bytes(*word);
    }
    for i in 16..80 {
        w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
    }
    let [mut a, mut b, mut c, mut d, mut e] = *state;
    for (i, word) in w.iter().enumerate() {
        let (f, k) = match i {
            0..=19 => ((b & c) | (!b & d), 0x5A82_7999),
            20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
            40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
            _ => (b ^ c ^ d, 0xCA62_C1D6),
        };
        let t = a
            .rotate_left(5)
            .wrapping_add(f)
            .wrapping_add(e)
            .wrapping_add(k)
            .wrapping_add(*word);
        e = d;
        d = c;
        c = b.rotate_left(30);
        b = a;
        a = t;
    }
    for (s, v) in state.iter_mut().zip([a, b, c, d, e]) {
        *s = s.wrapping_add(v);
    }
}

pub struct Sha256 {
    state: [u32; 8],
    blocks: Blocks,
}

const K256: [u32; 64] = [
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

impl Sha256 {
    pub fn new() -> Sha256 {
        Sha256 {
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
            blocks: Blocks::new(),
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        let state = &mut self.state;
        self.blocks
            .update(data, |block| sha256_compress(state, block));
    }

    pub fn finish(mut self) -> [u8; 32] {
        let state = &mut self.state;
        self.blocks.finish(|block| sha256_compress(state, block));
        let mut out = [0; 32];
        for (chunk, word) in out.as_chunks_mut::<4>().0.iter_mut().zip(self.state) {
            chunk.copy_from_slice(&word.to_be_bytes());
        }
        out
    }
}

#[expect(
    clippy::many_single_char_names,
    reason = "named as FIPS 180-4 names them"
)]
fn sha256_compress(state: &mut [u32; 8], block: &[u8; 64]) {
    let mut w = [0u32; 64];
    for (i, word) in block.as_chunks::<4>().0.iter().enumerate() {
        w[i] = u32::from_be_bytes(*word);
    }
    for i in 16..64 {
        let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
        let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16]
            .wrapping_add(s0)
            .wrapping_add(w[i - 7])
            .wrapping_add(s1);
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;
    for i in 0..64 {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ (!e & g);
        let t1 = h
            .wrapping_add(s1)
            .wrapping_add(ch)
            .wrapping_add(K256[i])
            .wrapping_add(w[i]);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let t2 = s0.wrapping_add(maj);
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }
    for (s, v) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
        *s = s.wrapping_add(v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha1(data: &[u8]) -> String {
        let mut sha = Sha1::new();
        sha.update(data);
        to_hex(&sha.finish())
    }

    fn sha256(data: &[u8]) -> String {
        let mut sha = Sha256::new();
        sha.update(data);
        to_hex(&sha.finish())
    }

    #[test]
    fn sha1_matches_the_standard_vectors() {
        assert_eq!(sha1(b""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(sha1(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(
            sha1(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "84983e441c3bd26ebaae4aa1f95129e5e54670f1"
        );
        assert_eq!(
            sha1(&vec![b'a'; 1_000_000]),
            "34aa973cd4c4daa4f61eeb2bdbad27316534016f"
        );
    }

    #[test]
    fn sha1_gives_the_same_answer_however_it_is_fed() {
        let data: Vec<u8> = (0..1000u32).map(|n| (n * 7).to_le_bytes()[0]).collect();
        for split in [0, 1, 55, 56, 63, 64, 65, 999] {
            let mut sha = Sha1::new();
            sha.update(&data[..split]);
            sha.update(&data[split..]);
            assert_eq!(to_hex(&sha.finish()), sha1(&data), "split at {split}");
        }
    }

    #[test]
    fn sha256_matches_the_standard_vectors() {
        assert_eq!(
            sha256(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn objects_are_named_as_git_names_them() {
        // `printf hello | git hash-object --stdin`
        assert_eq!(
            Oid::of("blob", b"hello").hex(),
            "b6fc4c620b67d95f953a5c1c1230aaab5db5a1b0"
        );
        assert_eq!(
            Oid::of("blob", b"").hex(),
            "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391"
        );
    }

    #[test]
    fn hex_round_trips() {
        let oid = Oid::of("blob", b"hello");
        assert_eq!(Oid::from_hex(oid.hex().as_bytes()), Some(oid));
        assert_eq!(Oid::from_hex(b"zz"), None);
    }
}
