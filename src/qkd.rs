// Copyright (C) 2026 CRV Science Network (https://crvscience.com)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://gnu.org>.

//! Quantum Key Distribution (QKD) & Bell-Inequality Security Suite:
//! - K1: BB84 Prepare-and-Measure (Bennett & Brassard 1984) + Cascade parity reconciliation + Toeplitz privacy amplification
//! - K2: E91 Entanglement-Based QKD & CHSH Bell Violation (Ekert 1991, Phys. Rev. Lett. 67, 661)
//! - K3: B92 Two-State Non-Orthogonal QKD & POVM Conclusive Erasure Sifting (Bennett 1992, Phys. Rev. Lett. 68, 3121)

use crate::complex::binary_entropy;
use crate::rng_sha256::{sha256_hex, DeterministicRng};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bb84PulseTrace {
    pub index: usize,
    pub alice_bit: u8,
    pub alice_basis: char,
    pub eve_intercepted: bool,
    pub eve_basis: char,
    pub bob_basis: char,
    pub bob_bit: u8,
    pub sifted_keep: bool,
    pub bit_error: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bb84SimulationReport {
    pub total_pulses: usize,
    pub eve_intercept_rate: f64,
    pub channel_flip_noise: f64,
    pub sifted_len: usize,
    pub sifted_errors: usize,
    pub observed_qber: f64,
    pub theoretical_qber: f64,
    pub shor_preskill_rate: f64,
    pub channel_secure: bool,
    pub reconciled_len: usize,
    pub leaked_parity_bits: usize,
    pub residual_mismatches_after_cascade: usize,
    pub final_amplified_key_bits: usize,
    pub alice_final_key_hex: String,
    pub bob_final_key_hex: String,
    pub keys_identical: bool,
    pub sample_pulses: Vec<Bb84PulseTrace>,
    /// `(eve_fraction, observed_like_qber_pct, asymptotic_key_rate)` curve.
    pub security_curve: Vec<(f64, f64, f64)>,
}

pub fn simulate_bb84(
    total_pulses: usize,
    eve_intercept_rate: f64,
    channel_flip_noise: f64,
    seed: u64,
) -> Bb84SimulationReport {
    let n = total_pulses.clamp(64, 8192);
    let p_eve = eve_intercept_rate.clamp(0.0, 1.0);
    let p_ch = channel_flip_noise.clamp(0.0, 0.35);
    let mut rng = DeterministicRng::from_seed(seed);

    let mut alice_sifted = Vec::with_capacity(n / 2 + 32);
    let mut bob_sifted = Vec::with_capacity(n / 2 + 32);
    let mut sample_pulses = Vec::with_capacity(48);

    for idx in 0..n {
        let a_bit = u8::from(rng.next_bool());
        let a_basis = rng.next_bool(); // false = '+' (Z), true = 'x' (X)
        let eve_acts = rng.next_f64() < p_eve;
        let e_basis = rng.next_bool();

        // State arriving at Bob after optional Eve intercept-resend
        let (tx_bit, tx_basis) = if eve_acts {
            let eve_meas = if e_basis == a_basis {
                a_bit
            } else {
                u8::from(rng.next_bool())
            };
            (eve_meas, e_basis)
        } else {
            (a_bit, a_basis)
        };

        let b_basis = rng.next_bool();
        let mut b_bit = if b_basis == tx_basis {
            tx_bit
        } else {
            u8::from(rng.next_bool())
        };
        if rng.next_f64() < p_ch {
            b_bit ^= 1;
        }

        let keep = a_basis == b_basis;
        let err = keep && (a_bit != b_bit);
        if keep {
            alice_sifted.push(a_bit);
            bob_sifted.push(b_bit);
        }

        if sample_pulses.len() < 48 {
            sample_pulses.push(Bb84PulseTrace {
                index: idx,
                alice_bit: a_bit,
                alice_basis: if a_basis { 'x' } else { '+' },
                eve_intercepted: eve_acts,
                eve_basis: if eve_acts {
                    if e_basis { 'x' } else { '+' }
                } else {
                    '-'
                },
                bob_basis: if b_basis { 'x' } else { '+' },
                bob_bit: b_bit,
                sifted_keep: keep,
                bit_error: err,
            });
        }
    }

    let sifted_len = alice_sifted.len();
    let sifted_errors = alice_sifted
        .iter()
        .zip(bob_sifted.iter())
        .filter(|(a, b)| a != b)
        .count();
    let observed_qber = if sifted_len > 0 {
        (sifted_errors as f64) / (sifted_len as f64)
    } else {
        0.0
    };

    // Exact theoretical QBER: Eve introduces 0.25 * p_eve flip probability, composed with symmetric channel flip p_ch
    let q_eve = 0.25 * p_eve;
    let theoretical_qber = q_eve * (1.0 - p_ch) + (1.0 - q_eve) * p_ch;
    let shor_preskill_rate = (1.0 - 2.0 * binary_entropy(observed_qber)).max(0.0);
    // Shor-Preskill asymptotic security threshold h2(QBER) < 0.5 => QBER < ~11.00%
    let channel_secure = observed_qber < 0.11 && sifted_len >= 32;

    // Two-pass Cascade binary parity-block reconciliation on sifted key
    let mut bob_corr = bob_sifted.clone();
    let mut leaked_parity_bits = 0usize;
    for &block_size in &[8usize, 13usize, 5usize] {
        let mut start = 0;
        while start < sifted_len {
            let end = (start + block_size).min(sifted_len);
            leaked_parity_bits += 1;
            let pa: u8 = alice_sifted[start..end].iter().fold(0, |acc, &x| acc ^ x);
            let pb: u8 = bob_corr[start..end].iter().fold(0, |acc, &x| acc ^ x);
            if pa != pb {
                // Binary search parity bisection to isolate and flip mismatched bit
                let mut lo = start;
                let mut hi = end;
                while hi - lo > 1 {
                    let mid = (lo + hi) / 2;
                    leaked_parity_bits += 1;
                    let l_pa: u8 = alice_sifted[lo..mid].iter().fold(0, |acc, &x| acc ^ x);
                    let l_pb: u8 = bob_corr[lo..mid].iter().fold(0, |acc, &x| acc ^ x);
                    if l_pa != l_pb {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                }
                bob_corr[lo] ^= 1;
            }
            start = end;
        }
    }

    let residual_mismatches = alice_sifted
        .iter()
        .zip(bob_corr.iter())
        .filter(|(a, b)| a != b)
        .count();

    // Toeplitz universal-2 hash privacy amplification shrinking reconciled bits to secure fraction
    let target_bits = if channel_secure {
        let raw_budget = ((sifted_len as f64) * shor_preskill_rate).floor() as usize;
        raw_budget.saturating_sub(16).min(256)
    } else {
        0
    };

    let (alice_final_key_hex, bob_final_key_hex, keys_identical) = if target_bits >= 8 {
        let out_len = (target_bits / 8) * 8;
        let mut toeplitz = vec![0u8; sifted_len + out_len];
        for b in &mut toeplitz {
            *b = u8::from(rng.next_bool());
        }
        let hash_bits = |input: &[u8]| -> Vec<u8> {
            let mut out = Vec::with_capacity(out_len);
            for r in 0..out_len {
                let mut acc = 0u8;
                for c in 0..input.len() {
                    acc ^= toeplitz[r + c] & input[c];
                }
                out.push(acc);
            }
            out
        };
        let ka = hash_bits(&alice_sifted);
        let kb = hash_bits(&bob_corr);
        let same = ka == kb;
        (bits_to_hex(&ka), bits_to_hex(&kb), same)
    } else {
        (
            "ABORTED (QBER >= 11.0% threshold)".to_string(),
            "ABORTED (QBER >= 11.0% threshold)".to_string(),
            false,
        )
    };

    let mut security_curve = Vec::with_capacity(51);
    for i in 0..=50 {
        let ef = (i as f64) * 0.02;
        let q_th = (0.25 * ef) * (1.0 - p_ch) + (1.0 - 0.25 * ef) * p_ch;
        let rate_th = (1.0 - 2.0 * binary_entropy(q_th)).max(0.0);
        security_curve.push((ef * 100.0, q_th * 100.0, rate_th));
    }

    let _ = sha256_hex(alice_final_key_hex.as_bytes());

    Bb84SimulationReport {
        total_pulses: n,
        eve_intercept_rate: p_eve,
        channel_flip_noise: p_ch,
        sifted_len,
        sifted_errors,
        observed_qber,
        theoretical_qber,
        shor_preskill_rate,
        channel_secure,
        reconciled_len: sifted_len,
        leaked_parity_bits,
        residual_mismatches_after_cascade: residual_mismatches,
        final_amplified_key_bits: (target_bits / 8) * 8,
        alice_final_key_hex,
        bob_final_key_hex,
        keys_identical,
        sample_pulses,
        security_curve,
    }
}

fn bits_to_hex(bits: &[u8]) -> String {
    let mut out = String::with_capacity(bits.len() / 4);
    for chunk in bits.chunks(8) {
        let mut byte = 0u8;
        for (i, &b) in chunk.iter().enumerate() {
            byte |= (b & 1) << (7 - i);
        }
        use std::fmt::Write;
        let _ = write!(&mut out, "{:02x}", byte);
    }
    out
}

/// E91 Entanglement-Based QKD & CHSH Bell Test (Ekert 1991).
/// Alice angles: a1 = 0, a2 = pi/4, a3 = pi/2
/// Bob angles:   b1 = pi/4, b2 = pi/2, b3 = 3*pi/4
/// Matching orientations (a2, b1) = pi/4 and (a3, b2) = pi/2 yield anti-correlated sifted key;
/// Mismatched orientations (a1, b1), (a1, b3), (a3, b1), (a3, b3) evaluate CHSH S!
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct E91SimulationReport {
    pub entangled_pairs: usize,
    pub eve_decoherence_fraction: f64,
    pub detector_visibility: f64,
    pub correlation_matrix: [[f64; 3]; 3],
    pub chsh_s_observed: f64,
    pub chsh_s_theoretical: f64,
    pub bell_violated: bool,
    pub sifted_key_len: usize,
    pub sifted_qber: f64,
    /// `(eve_fraction_pct, abs_chsh_s_theoretical)` curve showing 2*sqrt(2) -> sqrt(2) drop across |S| = 2.0.
    pub bell_curve: Vec<(f64, f64)>,
}

pub fn simulate_e91(
    entangled_pairs: usize,
    eve_decoherence_fraction: f64,
    detector_visibility: f64,
    seed: u64,
) -> E91SimulationReport {
    let n = entangled_pairs.clamp(200, 20_000);
    let p_eve = eve_decoherence_fraction.clamp(0.0, 1.0);
    let vis = detector_visibility.clamp(0.5, 1.0);
    let mut rng = DeterministicRng::from_seed(seed);

    let alice_angles = [0.0, PI / 4.0, PI / 2.0];
    let bob_angles = [PI / 4.0, PI / 2.0, 3.0 * PI / 4.0];

    let mut corr_sum = [[0.0_f64; 3]; 3];
    let mut corr_cnt = [[0usize; 3]; 3];
    let mut sifted_len = 0usize;
    let mut sifted_err = 0usize;

    // Singlet |Psi^-> = (|01> - |10>)/sqrt(2):
    // Coherent quantum expectation E_singlet(a, b) = -cos(a - b).
    // If Eve intercepts and measures in a random polarization angle phi_E uniformly in [0, pi),
    // the collapsed product state gives E_intercept(a, b) = -0.5 * cos(a - b)!
    for _ in 0..n {
        let ia = rng.next_usize(3);
        let ib = rng.next_usize(3);
        let theta_a = alice_angles[ia];
        let theta_b = bob_angles[ib];

        let (a_spin, b_spin) = if rng.next_f64() < p_eve {
            // Eve measures singlet along random axis phi_e, collapsing pair to opposite spin states
            let phi_e = rng.next_f64() * PI;
            let e_out: f64 = if rng.next_bool() { 1.0 } else { -1.0 };
            // Alice receives state along phi_e (+e_out), Bob receives (-e_out)
            let p_a_plus = 0.5 * (1.0 + vis * e_out * (theta_a - phi_e).cos());
            let p_b_plus = 0.5 * (1.0 - vis * e_out * (theta_b - phi_e).cos());
            let sa = if rng.next_f64() < p_a_plus { 1.0 } else { -1.0 };
            let sb = if rng.next_f64() < p_b_plus { 1.0 } else { -1.0 };
            (sa, sb)
        } else {
            // Intact Werner singlet with visibility `vis`: E(a, b) = -vis * cos(theta_a - theta_b)
            let sa = if rng.next_bool() { 1.0 } else { -1.0 };
            let target_corr = -vis * (theta_a - theta_b).cos();
            // P(sb == sa) = (1 + target_corr)/2
            let p_same = (0.5 * (1.0 + target_corr)).clamp(0.0, 1.0);
            let sb = if rng.next_f64() < p_same { sa } else { -sa };
            (sa, sb)
        };

        corr_sum[ia][ib] += a_spin * b_spin;
        corr_cnt[ia][ib] += 1;

        // Matching orientations: (ia=1, ib=0 => pi/4) or (ia=2, ib=1 => pi/2)
        if (ia == 1 && ib == 0) || (ia == 2 && ib == 1) {
            sifted_len += 1;
            let a_bit = u8::from(a_spin < 0.0);
            let b_bit = u8::from(b_spin > 0.0); // Bob inverts anti-correlated singlet readout
            if a_bit != b_bit {
                sifted_err += 1;
            }
        }
    }

    let mut correlation_matrix = [[0.0_f64; 3]; 3];
    for r in 0..3 {
        for c in 0..3 {
            if corr_cnt[r][c] > 0 {
                correlation_matrix[r][c] = corr_sum[r][c] / (corr_cnt[r][c] as f64);
            }
        }
    }

    // Standard singlet CHSH combination on (a1,b1), (a1,b3), (a3,b1), (a3,b3):
    // E(0, pi/4) = -1/sqrt(2), E(0, 3pi/4) = +1/sqrt(2), E(pi/2, pi/4) = -1/sqrt(2), E(pi/2, 3pi/4) = -1/sqrt(2)
    // S = E(a1, b1) - E(a1, b3) + E(a3, b1) + E(a3, b3) => -4 / sqrt(2) = -2*sqrt(2) -> |S| = 2*sqrt(2)!
    let s_raw = correlation_matrix[0][0] - correlation_matrix[0][2]
        + correlation_matrix[2][0]
        + correlation_matrix[2][2];
    let chsh_s_observed = s_raw.abs();
    let chsh_s_theoretical = 2.0 * std::f64::consts::SQRT_2 * vis * (1.0 - 0.5 * p_eve);
    let bell_violated = chsh_s_observed > 2.0;
    let sifted_qber = if sifted_len > 0 {
        (sifted_err as f64) / (sifted_len as f64)
    } else {
        0.0
    };

    let mut bell_curve = Vec::with_capacity(51);
    for i in 0..=50 {
        let ef = (i as f64) * 0.02;
        let s_th = 2.0 * std::f64::consts::SQRT_2 * vis * (1.0 - 0.5 * ef);
        bell_curve.push((ef * 100.0, s_th));
    }

    E91SimulationReport {
        entangled_pairs: n,
        eve_decoherence_fraction: p_eve,
        detector_visibility: vis,
        correlation_matrix,
        chsh_s_observed,
        chsh_s_theoretical,
        bell_violated,
        sifted_key_len: sifted_len,
        sifted_qber,
        bell_curve,
    }
}

/// B92 Two-State Non-Orthogonal QKD (Bennett 1992, Phys. Rev. Lett. 68, 3121):
/// Alice sends `|psi_0> = |0>` (bit 0) or `|psi_1> = cos(theta)|0> + sin(theta)|1>` (bit 1),
/// with state overlap `|<psi_0|psi_1>| = cos(theta)`.
/// Bob randomly projects onto `P_0^perp = |1><1|` (which never clicks on `|psi_0>`, conclusively revealing bit 1!)
/// or `P_1^perp = |psi_1^perp><psi_1^perp|` (which never clicks on `|psi_1>`, conclusively revealing bit 0!).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct B92SimulationReport {
    pub total_pulses: usize,
    pub state_angle_deg: f64,
    pub state_overlap_cos: f64,
    pub theoretical_conclusive_rate: f64,
    pub idp_usd_max_success: f64,
    pub eve_intercept_rate: f64,
    pub sifted_conclusive_len: usize,
    pub observed_qber: f64,
    /// `(angle_deg, bob_conclusive_rate, idp_usd_bound)` across `5..=85` deg.
    pub angle_tradeoff_curve: Vec<(f64, f64, f64)>,
}

pub fn simulate_b92(
    total_pulses: usize,
    state_angle_deg: f64,
    eve_intercept_rate: f64,
    seed: u64,
) -> B92SimulationReport {
    let n = total_pulses.clamp(100, 10_000);
    let deg = state_angle_deg.clamp(5.0, 85.0);
    let theta = deg * PI / 180.0;
    let overlap = theta.cos();
    let sin2 = theta.sin().powi(2);
    // Bob chooses randomly (1/2) between the two orthogonal complement tests; when he picks the right complement (1/2),
    // it clicks with probability sin^2(theta) => overall conclusive sifting rate = 0.5 * sin^2(theta)
    let theoretical_conclusive_rate = 0.5 * sin2;
    // Ivanovic-Dieks-Peres optimal Unambiguous State Discrimination bound: P_USD = 1 - |<psi_0|psi_1>| = 1 - cos(theta)
    let idp_usd_max_success = 1.0 - overlap;

    let p_eve = eve_intercept_rate.clamp(0.0, 1.0);
    let mut rng = DeterministicRng::from_seed(seed);
    let mut conclusive_len = 0usize;
    let mut errors = 0usize;

    for _ in 0..n {
        let a_bit = u8::from(rng.next_bool());
        // If Eve performs opaque intercept-resend using Bob's B92 complement projectors and resends a random bit on inconclusive:
        let tx_bit = if rng.next_f64() < p_eve {
            let eve_test = u8::from(rng.next_bool());
            let eve_conclusive = (eve_test != a_bit) && (rng.next_f64() < sin2);
            if eve_conclusive {
                a_bit
            } else {
                u8::from(rng.next_bool())
            }
        } else {
            a_bit
        };

        // Bob chooses test_basis in {0, 1}:
        // test_basis = 0 tests P_1^perp (conclusive click only possible if tx_bit == 0, with prob sin^2(theta))
        // test_basis = 1 tests P_0^perp (conclusive click only possible if tx_bit == 1, with prob sin^2(theta))
        let bob_test = u8::from(rng.next_bool());
        let clicks = (bob_test == tx_bit) && (rng.next_f64() < sin2);
        if clicks {
            conclusive_len += 1;
            if bob_test != a_bit {
                errors += 1;
            }
        }
    }

    let observed_qber = if conclusive_len > 0 {
        (errors as f64) / (conclusive_len as f64)
    } else {
        0.0
    };

    let mut angle_tradeoff_curve = Vec::with_capacity(81);
    for d_i in 5..=85 {
        let th = (d_i as f64) * PI / 180.0;
        angle_tradeoff_curve.push((d_i as f64, 0.5 * th.sin().powi(2), 1.0 - th.cos()));
    }

    B92SimulationReport {
        total_pulses: n,
        state_angle_deg: deg,
        state_overlap_cos: overlap,
        theoretical_conclusive_rate,
        idp_usd_max_success,
        eve_intercept_rate: p_eve,
        sifted_conclusive_len: conclusive_len,
        observed_qber,
        angle_tradeoff_curve,
    }
}
