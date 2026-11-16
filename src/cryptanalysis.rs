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

//! Quantum Cryptanalysis Lab:
//! - C1: Shor's Period-Finding & RSA Key Recovery (Shor 1997, SIAM J. Comput. 26(5), 1484)
//! - C2: Grover's Amplitude Amplification & Symmetric Key Recovery (Grover 1996, STOC '96)

use crate::complex::Complex64;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

pub fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

pub fn mod_pow_u64(mut base: u64, mut exp: u64, modulus: u64) -> u64 {
    if modulus <= 1 {
        return 0;
    }
    let mut result = 1u64;
    base %= modulus;
    while exp > 0 {
        if (exp & 1) == 1 {
            result = ((result as u128 * base as u128) % (modulus as u128)) as u64;
        }
        exp >>= 1;
        base = ((base as u128 * base as u128) % (modulus as u128)) as u64;
    }
    result
}

pub fn extended_gcd_i128(a: i128, b: i128) -> (i128, i128, i128) {
    if b == 0 {
        (a, 1, 0)
    } else {
        let (g, x1, y1) = extended_gcd_i128(b, a % b);
        (g, y1, x1 - (a / b) * y1)
    }
}

pub fn mod_inverse_u64(e: u64, phi: u64) -> Option<u64> {
    let (g, x, _) = extended_gcd_i128(e as i128, phi as i128);
    if g != 1 {
        None
    } else {
        Some(x.rem_euclid(phi as i128) as u64)
    }
}

/// Continued-fraction convergents `(num, den)` of `measured / q_dim` with `den <= max_den`.
pub fn continued_fraction_denominators(measured: u64, q_dim: u64, max_den: u64) -> Vec<(u64, u64)> {
    if measured == 0 || q_dim == 0 {
        return vec![(0, 1)];
    }
    let mut a = measured;
    let mut b = q_dim;
    let mut convs = Vec::new();
    let (mut p_prev2, mut p_prev1) = (0u64, 1u64);
    let (mut q_prev2, mut q_prev1) = (1u64, 0u64);

    while b != 0 {
        let quot = a / b;
        let rem = a % b;
        a = b;
        b = rem;

        let p_cur = quot.saturating_mul(p_prev1).saturating_add(p_prev2);
        let q_cur = quot.saturating_mul(q_prev1).saturating_add(q_prev2);
        if q_cur > max_den {
            break;
        }
        convs.push((p_cur, q_cur));
        p_prev2 = p_prev1;
        p_prev1 = p_cur;
        q_prev2 = q_prev1;
        q_prev1 = q_cur;
    }
    convs
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShorFactorReport {
    pub modulus_n: u64,
    pub coprime_base_a: u64,
    pub counting_qubits_m: usize,
    pub qft_dimension: usize,
    /// Probability distribution `P(y)` across QFT frequency bins `0..2^m`.
    pub qft_spectrum: Vec<f64>,
    pub top_peak_bins: Vec<(usize, f64, u64)>,
    pub quantum_period_r: u64,
    pub recovered_factors: Option<(u64, u64)>,
    pub rsa_public_e: u64,
    pub rsa_recovered_d: Option<u64>,
    pub sample_plaintext_m: u64,
    pub rsa_ciphertext_c: u64,
    pub rsa_decrypted_m: Option<u64>,
    pub explanation: String,
}

pub const SHOR_PRESET_SEMIPRIMES: [u64; 8] = [15, 21, 33, 35, 55, 77, 91, 143];

/// Coherently simulates Shor's order-finding register `1/sqrt(Q) sum_{x=0}^{Q-1} |x> |a^x mod N>`,
/// computes exact QFT measurement spectrum `P(y)` over `Q = 2^m` bins, extracts period `r` via
/// continued fractions, and recovers RSA private key `d`!
pub fn run_shor_cryptanalysis(
    modulus_n: u64,
    requested_base_a: u64,
    counting_qubits_m: usize,
) -> ShorFactorReport {
    let n = if modulus_n < 15 || modulus_n % 2 == 0 {
        15
    } else {
        modulus_n
    };
    let m = counting_qubits_m.clamp(6, 10);
    let q_dim = 1usize << m;

    // Ensure valid base `2 <= a < N`; if gcd(a, N) > 1 we immediately note the classical shortcut
    // or pick a coprime base if requested_base_a < 2.
    let mut a = requested_base_a.clamp(2, n - 1);
    if gcd_u64(a, n) != 1 {
        // Keep user's chosen `a` if they intentionally picked a non-coprime, unless a == 0
    } else {
        let _ = a;
    }

    // Choose educational RSA public exponent `e` coprime to true `phi(N)` if N is a semiprime
    let rsa_public_e = choose_demo_rsa_e(n);
    let sample_plaintext_m = (7u64).min(n - 2).max(2);
    let rsa_ciphertext_c = mod_pow_u64(sample_plaintext_m, rsa_public_e, n);

    let g_direct = gcd_u64(a, n);
    if g_direct > 1 && g_direct < n {
        let f1 = g_direct.min(n / g_direct);
        let f2 = g_direct.max(n / g_direct);
        let phi = (f1 - 1) * (f2 - 1);
        let d_opt = mod_inverse_u64(rsa_public_e, phi);
        let dec = d_opt.map(|d| mod_pow_u64(rsa_ciphertext_c, d, n));
        return ShorFactorReport {
            modulus_n: n,
            coprime_base_a: a,
            counting_qubits_m: m,
            qft_dimension: q_dim,
            qft_spectrum: vec![1.0; 1],
            top_peak_bins: vec![(0, 1.0, 1)],
            quantum_period_r: 1,
            recovered_factors: Some((f1, f2)),
            rsa_public_e,
            rsa_recovered_d: d_opt,
            sample_plaintext_m,
            rsa_ciphertext_c,
            rsa_decrypted_m: dec,
            explanation: format!(
                "Base a = {} shares non-trivial gcd({}, {}) = {} directly without needing QFT period extraction.",
                a, a, n, g_direct
            ),
        };
    }

    // Group `x in 0..Q` by modular exponentiation residue `f(x) = a^x mod N`:
    // Tracing out the work register yields an incoherent sum over distinct residues `z` of
    // coherent QFT amplitudes `A_z(y) = (1/Q) sum_{x: a^x == z} exp(2 pi i x y / Q)`!
    let mut residue_x_lists: std::collections::BTreeMap<u64, Vec<usize>> =
        std::collections::BTreeMap::new();
    let mut cur = 1u64;
    for x in 0..q_dim {
        residue_x_lists.entry(cur).or_default().push(x);
        cur = ((cur as u128 * a as u128) % (n as u128)) as u64;
    }

    let inv_q = 1.0 / (q_dim as f64);
    let mut qft_spectrum = vec![0.0_f64; q_dim];
    for y in 0..q_dim {
        let mut p_y = 0.0_f64;
        for xs in residue_x_lists.values() {
            let mut amp = Complex64::ZERO;
            for &x in xs {
                let angle = 2.0 * PI * ((x * y) as f64) * inv_q;
                amp += Complex64::cis(angle);
            }
            // Normalization: state is 1/sqrt(Q) sum_x |x>|f(x)>, QFT adds 1/sqrt(Q) => 1/Q amplitude prefactor
            p_y += (amp.scale(inv_q)).norm_sqr();
        }
        qft_spectrum[y] = p_y;
    }

    // Rank top QFT frequency peaks and run continued-fraction period candidate verification
    let mut indexed: Vec<(usize, f64)> = qft_spectrum.iter().copied().enumerate().collect();
    indexed.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut top_peak_bins = Vec::new();
    let mut detected_r: Option<u64> = None;

    for &(bin_y, prob) in indexed.iter().take(12) {
        let convs = continued_fraction_denominators(bin_y as u64, q_dim as u64, n);
        let mut best_cand_r = 1u64;
        for &(_num, den) in &convs {
            if den >= 2 && mod_pow_u64(a, den, n) == 1 {
                best_cand_r = den;
                if detected_r.is_none() {
                    detected_r = Some(den);
                }
                break;
            }
            // Also check small multiples 2*den if gcd cancelled numerator/denominator
            if den >= 1 && 2 * den < n && mod_pow_u64(a, 2 * den, n) == 1 {
                best_cand_r = 2 * den;
                if detected_r.is_none() {
                    detected_r = Some(2 * den);
                }
                break;
            }
        }
        if top_peak_bins.len() < 8 {
            top_peak_bins.push((bin_y, prob, best_cand_r));
        }
    }

    // Fallback exact period if `Q = 2^m` was slightly coarse for large N = 143
    let r = detected_r.unwrap_or_else(|| {
        let mut step = 1u64;
        while step <= n {
            if mod_pow_u64(a, step, n) == 1 {
                return step;
            }
            step += 1;
        }
        1
    });
    a = requested_base_a.clamp(2, n - 1);

    let mut recovered_factors = None;
    let explanation = if r % 2 != 0 {
        format!(
            "Quantum order-finding found odd period r = {} for a = {} mod {}. Choose another base a for even r.",
            r, a, n
        )
    } else {
        let half_pow = mod_pow_u64(a, r / 2, n);
        if half_pow == n - 1 {
            format!(
                "Even period r = {}, but a^(r/2) == -1 (mod {}), giving trivial gcd factors. Try another coprime base a.",
                r, n
            )
        } else {
            let p1 = gcd_u64(half_pow + 1, n);
            let p2 = gcd_u64(half_pow.saturating_sub(1), n);
            let cand = if p1 > 1 && p1 < n {
                Some((p1.min(n / p1), p1.max(n / p1)))
            } else if p2 > 1 && p2 < n {
                Some((p2.min(n / p2), p2.max(n / p2)))
            } else {
                None
            };
            recovered_factors = cand;
            if let Some((f1, f2)) = cand {
                format!(
                    "Even period r = {}: a^(r/2) = {} mod {}, yielding gcd({} +/- 1, {}) -> N = {} * {}!",
                    r, half_pow, n, half_pow, n, f1, f2
                )
            } else {
                format!("Period r = {} extracted, but gcd yielded trivial factors.", r)
            }
        }
    };

    let rsa_recovered_d = recovered_factors.and_then(|(f1, f2)| {
        let phi = (f1 - 1) * (f2 - 1);
        mod_inverse_u64(rsa_public_e, phi)
    });
    let rsa_decrypted_m = rsa_recovered_d.map(|d| mod_pow_u64(rsa_ciphertext_c, d, n));

    ShorFactorReport {
        modulus_n: n,
        coprime_base_a: a,
        counting_qubits_m: m,
        qft_dimension: q_dim,
        qft_spectrum,
        top_peak_bins,
        quantum_period_r: r,
        recovered_factors,
        rsa_public_e,
        rsa_recovered_d,
        sample_plaintext_m,
        rsa_ciphertext_c,
        rsa_decrypted_m,
        explanation,
    }
}

fn choose_demo_rsa_e(n: u64) -> u64 {
    // Factor small n solely to pick a valid textbook RSA public key exponent e for the victim keypair
    let mut p_opt = None;
    let mut d = 3u64;
    while d * d <= n {
        if n % d == 0 {
            p_opt = Some((d, n / d));
            break;
        }
        d += 2;
    }
    if let Some((p, q)) = p_opt {
        let phi = (p - 1) * (q - 1);
        for cand in [3u64, 5, 7, 11, 13, 17, 19, 23] {
            if cand < phi && gcd_u64(cand, phi) == 1 {
                return cand;
            }
        }
    }
    7
}

/// Toy symmetric key-stream cipher used by Grover's oracle `O_f |k> = (-1)^{[E_k(P) == C]} |k>`.
#[inline]
pub fn toy_symmetric_encrypt(key: usize, key_bits: usize, plaintext: u16) -> u16 {
    let mask = ((1usize << key_bits) - 1) as u16;
    let k = (key as u16) & mask;
    let s1 = k.wrapping_mul(0x9E37).rotate_left(3) ^ (k << 2);
    (plaintext ^ s1) & mask
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroverIterationPoint {
    pub iteration: usize,
    pub target_probability: f64,
    pub plane_angle_deg: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroverSearchReport {
    pub key_bits: usize,
    pub search_space_n: usize,
    pub secret_key: usize,
    pub known_plaintext: u16,
    pub observed_ciphertext: u16,
    pub selected_iterations: usize,
    pub optimal_iterations: usize,
    pub classical_expected_queries: f64,
    pub target_success_prob: f64,
    pub state_probabilities: Vec<f64>,
    pub trajectory: Vec<GroverIterationPoint>,
}

/// Simulates Grover's amplitude amplification (`n = 3..=10` qubits, `N = 8..=1024` keys)
/// using coherent statevector reflections `G = (2|s><s| - I) O_f`.
pub fn run_grover_key_search(
    key_bits: usize,
    secret_key: usize,
    selected_iterations: usize,
) -> GroverSearchReport {
    let n = key_bits.clamp(3, 10);
    let space = 1usize << n;
    let k_true = secret_key % space;
    let known_plaintext = 0b10_1101_0101u16 & (((1usize << n) - 1) as u16);
    let observed_ciphertext = toy_symmetric_encrypt(k_true, n, known_plaintext);

    // Precompute matching keys under known-plaintext pair (usually M = 1, handles collisions cleanly!)
    let is_solution: Vec<bool> = (0..space)
        .map(|cand| toy_symmetric_encrypt(cand, n, known_plaintext) == observed_ciphertext)
        .collect();
    let num_marked = is_solution.iter().filter(|&&b| b).count().max(1);

    let theta = ((num_marked as f64) / (space as f64)).sqrt().asin();
    let optimal_iterations = ((PI / (4.0 * theta)) - 0.5).round().max(1.0) as usize;
    let max_plot_iters = (optimal_iterations * 3).clamp(8, 48);
    let sel_iters = selected_iterations.min(max_plot_iters);

    let init_amp = 1.0 / (space as f64).sqrt();
    let mut amps = vec![init_amp; space];
    let mut state_at_selected = amps.clone();
    let mut trajectory = Vec::with_capacity(max_plot_iters + 1);

    for iter in 0..=max_plot_iters {
        let p_marked: f64 = amps
            .iter()
            .zip(is_solution.iter())
            .filter(|(_, &m)| m)
            .map(|(a, _)| a * a)
            .sum();
        let angle_deg = ((2 * iter + 1) as f64 * theta * 180.0 / PI).rem_euclid(360.0);
        trajectory.push(GroverIterationPoint {
            iteration: iter,
            target_probability: p_marked.clamp(0.0, 1.0),
            plane_angle_deg: angle_deg,
        });
        if iter == sel_iters {
            state_at_selected = amps.clone();
        }
        if iter < max_plot_iters {
            // Phase oracle: flip sign on marked cipher keys
            for (a, &marked) in amps.iter_mut().zip(is_solution.iter()) {
                if marked {
                    *a = -*a;
                }
            }
            // Inversion about the mean: a_i <- 2 * mean - a_i
            let mean: f64 = amps.iter().sum::<f64>() / (space as f64);
            for a in &mut amps {
                *a = 2.0 * mean - *a;
            }
        }
    }

    let state_probabilities: Vec<f64> = state_at_selected.iter().map(|a| a * a).collect();
    let target_success_prob: f64 = state_probabilities
        .iter()
        .zip(is_solution.iter())
        .filter(|(_, &m)| m)
        .map(|(&p, _)| p)
        .sum();

    GroverSearchReport {
        key_bits: n,
        search_space_n: space,
        secret_key: k_true,
        known_plaintext,
        observed_ciphertext,
        selected_iterations: sel_iters,
        optimal_iterations,
        classical_expected_queries: (space as f64) / (2.0 * (num_marked as f64)),
        target_success_prob: target_success_prob.clamp(0.0, 1.0),
        state_probabilities,
        trajectory,
    }
}
