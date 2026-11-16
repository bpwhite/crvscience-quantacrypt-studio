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

//! NIST Post-Quantum Cryptography (PQC) Lattice & Rejection-Sampling Suite:
//! - C3: NIST FIPS 203 ML-KEM (Module-LWE Key-Encapsulation Mechanism) over `R_q = Z_q[X]/(X^n + 1)` (`q = 3329`)
//!       with both inspectable pedagogical ring (`n = 16, k = 2`) and full FIPS 203 ring dimensions (`n = 256, k in {2, 3, 4}`),
//!       centered binomial distribution `CBD_eta`, decryption noise distribution `v - s^T u`, and 2D Lattice CVP/LWE geometry.
//! - C4: NIST FIPS 204 ML-DSA Fiat-Shamir with Aborts rejection sampling (`z = y + c*s`, `||z||_inf < gamma_1 - beta`)
//!       paired with stateful hash-based Merkle-Lamport authentication (`crate::rng_sha256::MerkleLamportDemo`).

use crate::rng_sha256::{sha256_hex, DeterministicRng, MerkleLamportDemo};
use serde::{Deserialize, Serialize};

pub const ML_KEM_Q: i32 = 3329;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MlKemProfile {
    Pedagogical16,
    MlKem512,
    MlKem768,
    MlKem1024,
}

impl MlKemProfile {
    pub const ALL: [Self; 4] = [
        Self::Pedagogical16,
        Self::MlKem512,
        Self::MlKem768,
        Self::MlKem1024,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Pedagogical16 => "Inspectable Pedagogical Ring (n = 16, k = 2, q = 3329)",
            Self::MlKem512 => "FIPS 203 ML-KEM-512 Structure (n = 256, k = 2, q = 3329, NIST Cat 1)",
            Self::MlKem768 => "FIPS 203 ML-KEM-768 Structure (n = 256, k = 3, q = 3329, NIST Cat 3)",
            Self::MlKem1024 => "FIPS 203 ML-KEM-1024 Structure (n = 256, k = 4, q = 3329, NIST Cat 5)",
        }
    }

    pub fn ring_degree_n(self) -> usize {
        match self {
            Self::Pedagogical16 => 16,
            Self::MlKem512 | Self::MlKem768 | Self::MlKem1024 => 256,
        }
    }

    pub fn module_rank_k(self) -> usize {
        match self {
            Self::Pedagogical16 | Self::MlKem512 => 2,
            Self::MlKem768 => 3,
            Self::MlKem1024 => 4,
        }
    }

    pub fn default_eta(self) -> usize {
        match self {
            Self::Pedagogical16 | Self::MlKem768 | Self::MlKem1024 => 2,
            Self::MlKem512 => 3,
        }
    }
}

/// Polynomial in negacyclic ring `R_q = Z_q[X]/(X^n + 1)` with coefficients in `0..q`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RingPoly {
    pub n: usize,
    pub coeffs: Vec<i32>,
}

impl RingPoly {
    pub fn zero(n: usize) -> Self {
        Self {
            n,
            coeffs: vec![0; n],
        }
    }

    pub fn uniform_random(n: usize, rng: &mut DeterministicRng) -> Self {
        let coeffs = (0..n)
            .map(|_| (rng.next_u32() % (ML_KEM_Q as u32)) as i32)
            .collect();
        Self { n, coeffs }
    }

    /// Samples centered binomial distribution `CBD_eta`: sum of `eta` bits minus sum of `eta` bits, reduced mod `q`.
    pub fn sample_cbd(n: usize, eta: usize, rng: &mut DeterministicRng) -> Self {
        let mut coeffs = Vec::with_capacity(n);
        for _ in 0..n {
            let mut a = 0i32;
            let mut b = 0i32;
            for _ in 0..eta {
                a += i32::from(rng.next_bool());
                b += i32::from(rng.next_bool());
            }
            coeffs.push((a - b).rem_euclid(ML_KEM_Q));
        }
        Self { n, coeffs }
    }

    pub fn add(&self, rhs: &Self) -> Self {
        assert_eq!(self.n, rhs.n);
        Self {
            n: self.n,
            coeffs: self
                .coeffs
                .iter()
                .zip(rhs.coeffs.iter())
                .map(|(&a, &b)| (a + b).rem_euclid(ML_KEM_Q))
                .collect(),
        }
    }

    pub fn sub(&self, rhs: &Self) -> Self {
        assert_eq!(self.n, rhs.n);
        Self {
            n: self.n,
            coeffs: self
                .coeffs
                .iter()
                .zip(rhs.coeffs.iter())
                .map(|(&a, &b)| (a - b).rem_euclid(ML_KEM_Q))
                .collect(),
        }
    }

    /// Exact negacyclic multiplication `c(X) = a(X) * b(X) mod (X^n + 1, q)` (`X^n == -1`).
    pub fn mul_negacyclic(&self, rhs: &Self) -> Self {
        assert_eq!(self.n, rhs.n);
        let n = self.n;
        let mut out = vec![0i64; n];
        for i in 0..n {
            let ai = self.coeffs[i] as i64;
            if ai == 0 {
                continue;
            }
            for j in 0..n {
                let prod = ai * (rhs.coeffs[j] as i64);
                if i + j < n {
                    out[i + j] += prod;
                } else {
                    out[i + j - n] -= prod;
                }
            }
        }
        Self {
            n,
            coeffs: out
                .into_iter()
                .map(|v| v.rem_euclid(ML_KEM_Q as i64) as i32)
                .collect(),
        }
    }

    /// Maps coefficients from `0..q` to centered representative in `[-(q-1)/2, +(q-1)/2]`.
    pub fn centered_coeffs(&self) -> Vec<i32> {
        let half = ML_KEM_Q / 2;
        self.coeffs
            .iter()
            .map(|&c| {
                let m = c.rem_euclid(ML_KEM_Q);
                if m > half {
                    m - ML_KEM_Q
                } else {
                    m
                }
            })
            .collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MlKemSimulationReport {
    pub profile: MlKemProfile,
    pub ring_n: usize,
    pub rank_k: usize,
    pub modulus_q: i32,
    pub noise_eta: usize,
    pub quarter_q_decision_bound: i32,
    pub max_abs_decryption_noise: i32,
    pub rms_decryption_noise: f64,
    pub bit_mismatches: usize,
    pub decapsulation_exact: bool,
    pub sender_shared_secret_hex: String,
    pub receiver_shared_secret_hex: String,
    /// Centered decryption error `e_dec[i]` for each polynomial slot `0..n` (for residual bar plot!).
    pub centered_noise_residuals: Vec<i32>,
    /// 2D Lattice basis & LWE noisy target sample `(lattice_points, exact_b, noisy_t)` for visual intuition.
    pub lattice_2d_points: Vec<[f64; 2]>,
    pub lattice_2d_exact_target: [f64; 2],
    pub lattice_2d_noisy_lwe: [f64; 2],
    pub lattice_basis_b1: [f64; 2],
    pub lattice_basis_b2: [f64; 2],
}

pub fn run_ml_kem_lab(
    profile: MlKemProfile,
    noise_eta: usize,
    seed: u64,
) -> MlKemSimulationReport {
    let n = profile.ring_degree_n();
    let k = profile.module_rank_k();
    let eta = noise_eta.clamp(1, 24);
    let mut rng = DeterministicRng::from_seed(seed);

    // 1. KeyGen: public matrix A in R_q^{k x k}, secret s in CBD_eta^k, error e in CBD_eta^k, t = A s + e
    let mut mat_a = vec![vec![RingPoly::zero(n); k]; k];
    for r in 0..k {
        for c in 0..k {
            mat_a[r][c] = RingPoly::uniform_random(n, &mut rng);
        }
    }
    let s_vec: Vec<RingPoly> = (0..k)
        .map(|_| RingPoly::sample_cbd(n, eta, &mut rng))
        .collect();
    let e_vec: Vec<RingPoly> = (0..k)
        .map(|_| RingPoly::sample_cbd(n, eta, &mut rng))
        .collect();

    let mut t_vec = vec![RingPoly::zero(n); k];
    for r in 0..k {
        let mut acc = RingPoly::zero(n);
        for c in 0..k {
            acc = acc.add(&mat_a[r][c].mul_negacyclic(&s_vec[c]));
        }
        t_vec[r] = acc.add(&e_vec[r]);
    }

    // 2. Encapsulate random n-bit message `msg_bits`:
    //    r_ephem, e1 in CBD_eta^k, e2 in CBD_eta
    //    u = A^T r_ephem + e1
    //    v = t^T r_ephem + e2 + round(q/2) * msg_bits
    let msg_bits: Vec<u8> = (0..n).map(|_| u8::from(rng.next_bool())).collect();
    let half_q = (ML_KEM_Q + 1) / 2; // 1665
    let msg_poly = RingPoly {
        n,
        coeffs: msg_bits.iter().map(|&b| (b as i32) * half_q).collect(),
    };

    let r_vec: Vec<RingPoly> = (0..k)
        .map(|_| RingPoly::sample_cbd(n, eta, &mut rng))
        .collect();
    let e1_vec: Vec<RingPoly> = (0..k)
        .map(|_| RingPoly::sample_cbd(n, eta, &mut rng))
        .collect();
    let e2_poly = RingPoly::sample_cbd(n, eta, &mut rng);

    let mut u_vec = vec![RingPoly::zero(n); k];
    for c in 0..k {
        let mut acc = RingPoly::zero(n);
        for r in 0..k {
            acc = acc.add(&mat_a[r][c].mul_negacyclic(&r_vec[r]));
        }
        u_vec[c] = acc.add(&e1_vec[c]);
    }

    let mut v_poly = RingPoly::zero(n);
    for i in 0..k {
        v_poly = v_poly.add(&t_vec[i].mul_negacyclic(&r_vec[i]));
    }
    v_poly = v_poly.add(&e2_poly).add(&msg_poly);

    // 3. Decapsulate: w = v - s^T u = round(q/2)*msg + (e^T r + e2 - s^T e1)
    let mut s_dot_u = RingPoly::zero(n);
    for i in 0..k {
        s_dot_u = s_dot_u.add(&s_vec[i].mul_negacyclic(&u_vec[i]));
    }
    let w_poly = v_poly.sub(&s_dot_u);

    // Exact decryption noise polynomial `w - msg_poly`
    let noise_poly = w_poly.sub(&msg_poly);
    let centered_noise_residuals = noise_poly.centered_coeffs();
    let quarter_q = ML_KEM_Q / 4; // 832

    let mut dec_bits = Vec::with_capacity(n);
    for &wc in &w_poly.coeffs {
        // Closer to round(q/2) than to 0 mod q iff |wc - q/2| < q/4
        let dist_half = (wc - half_q).abs();
        dec_bits.push(u8::from(dist_half <= quarter_q));
    }

    let bit_mismatches = msg_bits
        .iter()
        .zip(dec_bits.iter())
        .filter(|(a, b)| a != b)
        .count();
    let max_abs_decryption_noise = centered_noise_residuals
        .iter()
        .map(|x| x.abs())
        .max()
        .unwrap_or(0);
    let rms_decryption_noise = (centered_noise_residuals
        .iter()
        .map(|&x| (x as f64) * (x as f64))
        .sum::<f64>()
        / (n as f64))
        .sqrt();

    let sender_shared_secret_hex = sha256_hex(&msg_bits);
    let receiver_shared_secret_hex = sha256_hex(&dec_bits);

    // 2D Lattice LWE visual grid: basis b1 = [4.0, 1.1], b2 = [1.3, 3.6], secret s = (2, -1)
    let b1 = [4.0_f64, 1.1_f64];
    let b2 = [1.3_f64, 3.6_f64];
    let mut lattice_2d_points = Vec::new();
    for i in -4..=4 {
        for j in -4..=4 {
            lattice_2d_points.push([
                (i as f64) * b1[0] + (j as f64) * b2[0],
                (i as f64) * b1[1] + (j as f64) * b2[1],
            ]);
        }
    }
    let exact_target = [2.0 * b1[0] - 1.0 * b2[0], 2.0 * b1[1] - 1.0 * b2[1]];
    let err_scale = (eta as f64) * 0.14;
    let dx = (centered_noise_residuals[0] as f64 / 40.0).clamp(-1.8, 1.8) + err_scale * 0.45;
    let dy = (centered_noise_residuals[1] as f64 / 40.0).clamp(-1.8, 1.8) - err_scale * 0.35;
    let noisy_lwe = [exact_target[0] + dx, exact_target[1] + dy];

    MlKemSimulationReport {
        profile,
        ring_n: n,
        rank_k: k,
        modulus_q: ML_KEM_Q,
        noise_eta: eta,
        quarter_q_decision_bound: quarter_q,
        max_abs_decryption_noise,
        rms_decryption_noise,
        bit_mismatches,
        decapsulation_exact: bit_mismatches == 0,
        sender_shared_secret_hex,
        receiver_shared_secret_hex,
        centered_noise_residuals,
        lattice_2d_points,
        lattice_2d_exact_target: exact_target,
        lattice_2d_noisy_lwe: noisy_lwe,
        lattice_basis_b1: b1,
        lattice_basis_b2: b2,
    }
}

/// NIST FIPS 204 ML-DSA (Dilithium) Fiat-Shamir with Aborts rejection-sampling lab + Merkle-Lamport signatures.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PqcSignatureReport {
    pub gamma1_mask_bound: i32,
    pub secret_s_bound_eta: i32,
    pub challenge_weight_tau: usize,
    pub beta_safety_margin: i32,
    pub total_signing_trials: usize,
    pub accepted_signatures: usize,
    pub rejected_aborts: usize,
    pub acceptance_rate: f64,
    pub expected_attempts_per_sig: f64,
    /// Histogram bins `(z_value, raw_unrejected_count, accepted_uniform_count)` showing why rejection hides `s`!
    pub z_distribution_bins: Vec<(i32, usize, usize)>,
    pub merkle_lamport: MerkleLamportDemo,
}

pub fn run_pqc_signature_lab(
    gamma1_mask_bound: i32,
    secret_eta: i32,
    challenge_tau: usize,
    tamper_message: bool,
    seed: u64,
) -> PqcSignatureReport {
    let gamma1 = gamma1_mask_bound.clamp(32, 512);
    let eta = secret_eta.clamp(1, 8);
    let tau = challenge_tau.clamp(1, 16);
    let beta = (tau as i32) * eta;
    let safe_limit = (gamma1 - beta).max(4);

    let mut rng = DeterministicRng::from_seed(seed);
    let trials = 2_400usize;
    let dim_check = 16usize; // 16-coefficient pedagogical slice so multi-coord rejection rate matches Dilithium ~20-50%

    let secret_s: Vec<i32> = (0..dim_check)
        .map(|_| (rng.next_usize((2 * eta + 1) as usize) as i32) - eta)
        .collect();

    let n_bins = 21usize;
    let bin_max = gamma1 + beta;
    let mut raw_hist = vec![0usize; n_bins];
    let mut acc_hist = vec![0usize; n_bins];

    let to_bin = |val: i32| -> usize {
        let norm = ((val + bin_max) as f64) / ((2 * bin_max).max(1) as f64);
        ((norm * (n_bins as f64)).floor() as isize).clamp(0, (n_bins - 1) as isize) as usize
    };

    let mut accepted = 0usize;
    for _ in 0..trials {
        // Mask vector y uniform in [-gamma1 + 1, gamma1]
        let mut z_vec = Vec::with_capacity(dim_check);
        for &s_i in &secret_s {
            let span = (2 * gamma1) as usize;
            let y_i = (rng.next_usize(span) as i32) - gamma1 + 1;
            // Challenge polynomial sparse sum `c * s` bounded by `beta` (biased positive on s_i != 0 without rejection!)
            let cs_i = s_i * ((rng.next_usize(tau + 1) as i32).min(tau as i32));
            let z_i = y_i + cs_i.clamp(-beta, beta);
            z_vec.push(z_i);
        }
        raw_hist[to_bin(z_vec[0])] += 1;

        if z_vec.iter().all(|&zi| zi.abs() < safe_limit) {
            accepted += 1;
            acc_hist[to_bin(z_vec[0])] += 1;
        }
    }

    let z_distribution_bins: Vec<(i32, usize, usize)> = (0..n_bins)
        .map(|b| {
            let center = -bin_max + ((2 * bin_max * (b as i32) + bin_max) / (n_bins as i32));
            (center, raw_hist[b], acc_hist[b])
        })
        .collect();

    let acceptance_rate = (accepted as f64) / (trials as f64);
    let expected_attempts = if acceptance_rate > 1e-6 {
        1.0 / acceptance_rate
    } else {
        trials as f64
    };

    let merkle_lamport = MerkleLamportDemo::run(
        seed ^ 0xA5A5_5A5A,
        3,
        2,
        "CRV-Science Quantum Telemetry Block #0042",
        tamper_message,
    );

    PqcSignatureReport {
        gamma1_mask_bound: gamma1,
        secret_s_bound_eta: eta,
        challenge_weight_tau: tau,
        beta_safety_margin: beta,
        total_signing_trials: trials,
        accepted_signatures: accepted,
        rejected_aborts: trials - accepted,
        acceptance_rate,
        expected_attempts_per_sig: expected_attempts,
        z_distribution_bins,
        merkle_lamport,
    }
}
