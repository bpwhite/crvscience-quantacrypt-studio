//! Pure safe-Rust deterministic CSPRNG (ChaCha20 stream + SplitMix64 seeder),
//! NIST FIPS 180-4 SHA-256 digest, Lamport One-Time Signatures, and Merkle authentication trees.

use serde::{Deserialize, Serialize};

/// Deterministic SplitMix64 seed expander + ChaCha20 CSPRNG for reproducible simulations and crypto labs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeterministicRng {
    state: [u32; 16],
    buffer: [u32; 16],
    word_idx: usize,
}

impl DeterministicRng {
    pub fn from_seed(seed: u64) -> Self {
        let mut sm = seed;
        let mut next_u64 = || -> u64 {
            sm = sm.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = sm;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        };

        let mut key_words = [0u32; 8];
        for pair in 0..4 {
            let w = next_u64();
            key_words[2 * pair] = w as u32;
            key_words[2 * pair + 1] = (w >> 32) as u32;
        }
        let nonce_w = next_u64();
        let state = [
            0x6170_7865,
            0x3320_646e,
            0x7962_2d32,
            0x6b20_6574,
            key_words[0],
            key_words[1],
            key_words[2],
            key_words[3],
            key_words[4],
            key_words[5],
            key_words[6],
            key_words[7],
            0,
            0,
            nonce_w as u32,
            (nonce_w >> 32) as u32,
        ];
        let mut rng = Self {
            state,
            buffer: [0u32; 16],
            word_idx: 16,
        };
        rng.refill();
        rng
    }

    #[inline]
    fn quarter_round(x: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
        x[a] = x[a].wrapping_add(x[b]);
        x[d] = (x[d] ^ x[a]).rotate_left(16);
        x[c] = x[c].wrapping_add(x[d]);
        x[b] = (x[b] ^ x[c]).rotate_left(12);
        x[a] = x[a].wrapping_add(x[b]);
        x[d] = (x[d] ^ x[a]).rotate_left(8);
        x[c] = x[c].wrapping_add(x[d]);
        x[b] = (x[b] ^ x[c]).rotate_left(7);
    }

    fn refill(&mut self) {
        let mut working = self.state;
        for _ in 0..10 {
            // Column rounds
            Self::quarter_round(&mut working, 0, 4, 8, 12);
            Self::quarter_round(&mut working, 1, 5, 9, 13);
            Self::quarter_round(&mut working, 2, 6, 10, 14);
            Self::quarter_round(&mut working, 3, 7, 11, 15);
            // Diagonal rounds
            Self::quarter_round(&mut working, 0, 5, 10, 15);
            Self::quarter_round(&mut working, 1, 6, 11, 12);
            Self::quarter_round(&mut working, 2, 7, 8, 13);
            Self::quarter_round(&mut working, 3, 4, 9, 14);
        }
        for i in 0..16 {
            self.buffer[i] = working[i].wrapping_add(self.state[i]);
        }
        self.state[12] = self.state[12].wrapping_add(1);
        if self.state[12] == 0 {
            self.state[13] = self.state[13].wrapping_add(1);
        }
        self.word_idx = 0;
    }

    pub fn next_u32(&mut self) -> u32 {
        if self.word_idx >= 16 {
            self.refill();
        }
        let val = self.buffer[self.word_idx];
        self.word_idx += 1;
        val
    }

    pub fn next_u64(&mut self) -> u64 {
        let lo = self.next_u32() as u64;
        let hi = self.next_u32() as u64;
        (hi << 32) | lo
    }

    /// Uniform f64 in [0.0, 1.0).
    pub fn next_f64(&mut self) -> f64 {
        const SCALE: f64 = 1.0 / ((1u64 << 53) as f64);
        ((self.next_u64() >> 11) as f64) * SCALE
    }

    pub fn next_bool(&mut self) -> bool {
        (self.next_u32() & 1) == 1
    }

    pub fn next_usize(&mut self, bound: usize) -> usize {
        if bound <= 1 {
            return 0;
        }
        (self.next_u64() % (bound as u64)) as usize
    }

    pub fn fill_bytes(&mut self, dest: &mut [u8]) {
        let mut idx = 0;
        while idx < dest.len() {
            let bytes = self.next_u32().to_le_bytes();
            let take = (dest.len() - idx).min(4);
            dest[idx..idx + take].copy_from_slice(&bytes[..take]);
            idx += take;
        }
    }
}

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// Pure safe-Rust NIST FIPS 180-4 SHA-256 implementation.
pub fn sha256(msg: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    let bit_len = (msg.len() as u64) * 8;
    let mut padded = Vec::with_capacity(msg.len() + 72);
    padded.extend_from_slice(msg);
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0x00);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[4 * i],
                chunk[4 * i + 1],
                chunk[4 * i + 2],
                chunk[4 * i + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA256_K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

pub fn sha256_hex(msg: &[u8]) -> String {
    let digest = sha256(msg);
    let mut s = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(&mut s, "{:02x}", b);
    }
    s
}

/// Computes bit-level Hamming distance between two 32-byte SHA-256 digests (0..=256).
pub fn sha256_hamming_distance(a: &[u8; 32], b: &[u8; 32]) -> u32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x ^ y).count_ones())
        .sum()
}

/// Compact educational Lamport One-Time Signature (32 message digest bits -> 32 preimage pairs)
/// bundled into a stateful Merkle root tree of `2^height` leaves (post-quantum hash signature model).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MerkleLamportDemo {
    pub tree_height: usize,
    pub leaf_index: usize,
    pub message: String,
    pub tampered_message: String,
    pub merkle_root_hex: String,
    pub leaf_pub_hash_hex: String,
    pub auth_path_hex: Vec<String>,
    pub valid_original: bool,
    pub valid_tampered: bool,
    pub avalanche_flipped_bits: u32,
}

impl MerkleLamportDemo {
    pub fn run(seed: u64, tree_height: usize, leaf_index: usize, message: &str, tamper_flip: bool) -> Self {
        let h = tree_height.clamp(2, 5);
        let num_leaves = 1usize << h;
        let idx = leaf_index % num_leaves;
        let mut rng = DeterministicRng::from_seed(seed);

        // Generate 64-bit educational Lamport keys per leaf (64 pairs of 16-byte preimages)
        let bits = 64usize;
        let mut sk_leaves = Vec::with_capacity(num_leaves);
        let mut leaf_hashes = Vec::with_capacity(num_leaves);

        for _leaf in 0..num_leaves {
            let mut pairs = Vec::with_capacity(bits);
            let mut pk_concat = Vec::with_capacity(bits * 64);
            for _b in 0..bits {
                let mut s0 = [0u8; 16];
                let mut s1 = [0u8; 16];
                rng.fill_bytes(&mut s0);
                rng.fill_bytes(&mut s1);
                let p0 = sha256(&s0);
                let p1 = sha256(&s1);
                pk_concat.extend_from_slice(&p0);
                pk_concat.extend_from_slice(&p1);
                pairs.push((s0, s1, p0, p1));
            }
            leaf_hashes.push(sha256(&pk_concat));
            sk_leaves.push(pairs);
        }

        // Build binary Merkle tree
        let mut levels = vec![leaf_hashes.clone()];
        while levels.last().map(|l| l.len()).unwrap_or(0) > 1 {
            let prev = levels.last().unwrap();
            let mut next_lvl = Vec::with_capacity(prev.len() / 2);
            for pair in prev.chunks_exact(2) {
                let mut combined = [0u8; 64];
                combined[..32].copy_from_slice(&pair[0]);
                combined[32..].copy_from_slice(&pair[1]);
                next_lvl.push(sha256(&combined));
            }
            levels.push(next_lvl);
        }
        let root = levels.last().unwrap()[0];

        // Sign `message` using leaf `idx` (first 64 bits of SHA-256(message))
        let msg_digest = sha256(message.as_bytes());
        let mut signature_preimages = Vec::with_capacity(bits);
        for bit_i in 0..bits {
            let byte_val = msg_digest[bit_i / 8];
            let bit_val = (byte_val >> (bit_i % 8)) & 1;
            let (s0, s1, _, _) = sk_leaves[idx][bit_i];
            signature_preimages.push(if bit_val == 0 { s0 } else { s1 });
        }

        // Collect Merkle authentication path
        let mut auth_path = Vec::with_capacity(h);
        let mut cur = idx;
        for lvl in 0..h {
            let sibling = cur ^ 1;
            auth_path.push(levels[lvl][sibling]);
            cur >>= 1;
        }

        let verify = |candidate_msg: &str| -> bool {
            let cand_digest = sha256(candidate_msg.as_bytes());
            let mut reconstructed_pk = Vec::with_capacity(bits * 64);
            for bit_i in 0..bits {
                let bit_val = (cand_digest[bit_i / 8] >> (bit_i % 8)) & 1;
                let revealed_hash = sha256(&signature_preimages[bit_i]);
                let (_, _, p0, p1) = sk_leaves[idx][bit_i];
                if bit_val == 0 {
                    if revealed_hash != p0 {
                        return false;
                    }
                    reconstructed_pk.extend_from_slice(&revealed_hash);
                    reconstructed_pk.extend_from_slice(&p1);
                } else {
                    if revealed_hash != p1 {
                        return false;
                    }
                    reconstructed_pk.extend_from_slice(&p0);
                    reconstructed_pk.extend_from_slice(&revealed_hash);
                }
            }
            let mut node = sha256(&reconstructed_pk);
            let mut pos = idx;
            for sib in &auth_path {
                let mut combined = [0u8; 64];
                if pos % 2 == 0 {
                    combined[..32].copy_from_slice(&node);
                    combined[32..].copy_from_slice(sib);
                } else {
                    combined[..32].copy_from_slice(sib);
                    combined[32..].copy_from_slice(&node);
                }
                node = sha256(&combined);
                pos >>= 1;
            }
            node == root
        };

        let tampered_message = if tamper_flip {
            format!("{} [tampered]", message)
        } else {
            message.to_string()
        };
        let tampered_digest = sha256(tampered_message.as_bytes());
        let avalanche_flipped_bits = sha256_hamming_distance(&msg_digest, &tampered_digest);

        let hex32 = |arr: &[u8; 32]| -> String {
            let mut s = String::with_capacity(64);
            for b in arr {
                use std::fmt::Write;
                let _ = write!(&mut s, "{:02x}", b);
            }
            s
        };

        Self {
            tree_height: h,
            leaf_index: idx,
            message: message.to_string(),
            tampered_message: tampered_message.clone(),
            merkle_root_hex: hex32(&root),
            leaf_pub_hash_hex: hex32(&leaf_hashes[idx]),
            auth_path_hex: auth_path.iter().map(hex32).collect(),
            valid_original: verify(message),
            valid_tampered: verify(&tampered_message),
            avalanche_flipped_bits,
        }
    }
}
