//! High-dimensional qudit (`d = 2..=8`) statevector & Weyl-Heisenberg optical gate engine
//! (Wang, Hu, Sanders & Kais 2020, Front. Phys. 8:589504):
//! - Generalized shift `X_d |k> = |(k + 1) mod d>` and clock `Z_d |k> = omega^k |k>` (`omega = e^{2 pi i / d}`)
//! - Weyl commutation relation `Z_d X_d = omega X_d Z_d`
//! - `d`-dimensional Quantum Fourier Transform `F_d |j> = (1/sqrt(d)) sum_k omega^{j k} |k>`
//! - Two-qudit `CSUM |j, k> = |j, (j + k) mod d>` producing maximally entangled `|Phi_d>` (`log2(d)` ebits)
//! - High-dimensional prepare-and-measure QKD key rate `R_d(Q)` vs. standard qubit BB84.

use crate::complex::{CMatrix, Complex64};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuditPreset {
    SingleFourierSuperposition,
    WeylClockShiftOrbit,
    BellMaximallyEntangledPair,
    ThreeQuditGhzRing,
}

impl QuditPreset {
    pub const ALL: [Self; 4] = [
        Self::BellMaximallyEntangledPair,
        Self::SingleFourierSuperposition,
        Self::WeylClockShiftOrbit,
        Self::ThreeQuditGhzRing,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::BellMaximallyEntangledPair => "2-Qudit Entangled Bell State |Phi_d> = F_d + CSUM",
            Self::SingleFourierSuperposition => "Single-Qudit Chiral QFT State F_d |m>",
            Self::WeylClockShiftOrbit => "Weyl-Heisenberg X_d^a Z_d^b Phase-Space Displacement",
            Self::ThreeQuditGhzRing => "3-Qudit High-Dimensional GHZ State (1/sqrt(d)) sum |j,j,j>",
        }
    }
}

/// Generalized `d x d` shift operator `X_d |k> = |(k + 1) mod d>`.
pub fn qudit_shift_x(d: usize) -> CMatrix {
    assert!((2..=12).contains(&d));
    let mut m = CMatrix::zeros(d);
    for k in 0..d {
        let next = (k + 1) % d;
        m.set(next, k, Complex64::ONE);
    }
    m
}

/// Generalized `d x d` clock operator `Z_d |k> = exp(2 pi i k / d) |k>`.
pub fn qudit_clock_z(d: usize) -> CMatrix {
    assert!((2..=12).contains(&d));
    let mut m = CMatrix::zeros(d);
    for k in 0..d {
        let phase = 2.0 * PI * (k as f64) / (d as f64);
        m.set(k, k, Complex64::cis(phase));
    }
    m
}

/// `d`-dimensional Quantum Fourier Transform `F_d` with `(F_d)_{r, c} = exp(2 pi i r c / d) / sqrt(d)`.
pub fn qudit_qft(d: usize) -> CMatrix {
    assert!((2..=12).contains(&d));
    let scale = 1.0 / (d as f64).sqrt();
    let mut m = CMatrix::zeros(d);
    for r in 0..d {
        for c in 0..d {
            let angle = 2.0 * PI * ((r * c) as f64) / (d as f64);
            m.set(r, c, Complex64::cis(angle).scale(scale));
        }
    }
    m
}

/// Verifies Frobenius norm `|| Z_d X_d - omega X_d Z_d ||_F` (exact zero up to floating-point precision).
pub fn weyl_commutation_residual(d: usize) -> f64 {
    let xd = qudit_shift_x(d);
    let zd = qudit_clock_z(d);
    let zx = zd.matmul(&xd);
    let omega = Complex64::cis(2.0 * PI / (d as f64));
    let omega_xz = xd.matmul(&zd).scale(omega);
    zx.data
        .iter()
        .zip(omega_xz.data.iter())
        .map(|(&a, &b)| (a - b).norm_sqr())
        .sum::<f64>()
        .sqrt()
}

/// Pure `n`-qudit register over local Hilbert space dimension `d` (`dim = d^n`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuditRegister {
    pub dimension_d: usize,
    pub num_qudits: usize,
    pub amps: Vec<Complex64>,
}

impl QuditRegister {
    pub fn zero_state(dimension_d: usize, num_qudits: usize) -> Self {
        let d = dimension_d.clamp(2, 8);
        let n = num_qudits.clamp(1, 4);
        let total = d.pow(n as u32);
        let mut amps = vec![Complex64::ZERO; total];
        amps[0] = Complex64::ONE;
        Self {
            dimension_d: d,
            num_qudits: n,
            amps,
        }
    }

    /// Encodes qudit digits `[q_0, q_1, ..., q_{n-1}]` (`q_i in 0..d`) into flat basis index.
    pub fn digits_to_index(&self, digits: &[usize]) -> usize {
        let mut idx = 0usize;
        let mut stride = 1usize;
        for &dig in digits.iter().take(self.num_qudits) {
            idx += (dig % self.dimension_d) * stride;
            stride *= self.dimension_d;
        }
        idx
    }

    pub fn index_to_digits(&self, mut idx: usize) -> Vec<usize> {
        let mut digits = Vec::with_capacity(self.num_qudits);
        for _ in 0..self.num_qudits {
            digits.push(idx % self.dimension_d);
            idx /= self.dimension_d;
        }
        digits
    }

    pub fn basis_label(&self, idx: usize) -> String {
        let digs = self.index_to_digits(idx);
        let s: String = digs
            .iter()
            .rev()
            .map(|d| char::from_digit(*d as u32, 10).unwrap_or('?'))
            .collect();
        format!("|{}\u{27E9}", s)
    }

    /// Applies single-qudit `d x d` operator `op` to `target` qudit (`0..num_qudits`).
    pub fn apply_single(&mut self, target: usize, op: &CMatrix) {
        assert!(target < self.num_qudits);
        assert_eq!(op.dim, self.dimension_d);
        let d = self.dimension_d;
        let stride = d.pow(target as u32);
        let block = stride * d;
        let total = self.amps.len();

        let mut base = 0;
        while base < total {
            for offset in 0..stride {
                let mut slice = vec![Complex64::ZERO; d];
                for k in 0..d {
                    slice[k] = self.amps[base + offset + k * stride];
                }
                let out = op.apply_vec(&slice);
                for k in 0..d {
                    self.amps[base + offset + k * stride] = out[k];
                }
            }
            base += block;
        }
    }

    /// Applies two-qudit controlled modular sum `CSUM |j, k> = |j, (j + k) mod d>`.
    pub fn apply_csum(&mut self, control: usize, target: usize) {
        assert!(control < self.num_qudits && target < self.num_qudits && control != target);
        let total = self.amps.len();
        let mut next = vec![Complex64::ZERO; total];
        for idx in 0..total {
            let amp = self.amps[idx];
            if amp.norm_sqr() == 0.0 {
                continue;
            }
            let mut digs = self.index_to_digits(idx);
            digs[target] = (digs[target] + digs[control]) % self.dimension_d;
            let dst = self.digits_to_index(&digs);
            next[dst] += amp;
        }
        self.amps = next;
    }

    /// Reduced density matrix of qudit `0` (`d x d`) by tracing out qudits `1..n`.
    pub fn reduced_qudit0_rho(&self) -> CMatrix {
        let d = self.dimension_d;
        let env_dim = self.amps.len() / d;
        let mut rho = CMatrix::zeros(d);
        for env in 0..env_dim {
            let base = env * d;
            for i in 0..d {
                let ai = self.amps[base + i];
                if ai.norm_sqr() == 0.0 {
                    continue;
                }
                for j in 0..d {
                    let aj = self.amps[base + j];
                    rho.add_at(i, j, ai * aj.conj());
                }
            }
        }
        rho
    }

    pub fn entanglement_ebits_qudit0(&self) -> f64 {
        if self.num_qudits < 2 {
            return 0.0;
        }
        self.reduced_qudit0_rho().von_neumann_entropy_bits()
    }
}

/// Asymptotic `d`-dimensional BB84/2-MUB secret key rate in bits per sifted photon (Sheridan & Scarani 2010):
/// `R_d(Q) = max(0, log2(d) - 2 * [ -Q log2(Q / (d - 1)) - (1 - Q) log2(1 - Q) ])` for `0 <= Q < (d-1)/d`.
pub fn qudit_qkd_secret_bits_per_sifted(d: usize, qber: f64) -> f64 {
    let df = d.max(2) as f64;
    let q_max = (df - 1.0) / df;
    let q = qber.clamp(0.0, q_max);
    if q <= 1e-15 {
        return df.log2();
    }
    if q >= q_max - 1e-12 {
        return 0.0;
    }
    let hd = -q * (q / (df - 1.0)).log2() - (1.0 - q) * (1.0 - q).log2();
    (df.log2() - 2.0 * hd).max(0.0)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuditLabReport {
    pub dimension_d: usize,
    pub preset: QuditPreset,
    pub shift_power_a: usize,
    pub clock_power_b: usize,
    pub basis_labels: Vec<String>,
    pub probabilities: Vec<f64>,
    pub phases_rad: Vec<f64>,
    pub entanglement_ebits: f64,
    pub max_ebits_for_d: f64,
    pub weyl_residual: f64,
    pub qft_is_unitary: bool,
    /// `(qber, r_d2_bits, r_d_current_bits)` across `qber in 0.00..=0.30`.
    pub qkd_capacity_curve: Vec<(f64, f64, f64)>,
    pub tolerated_qber_pct: f64,
}

pub fn run_qudit_lab(
    dimension_d: usize,
    preset: QuditPreset,
    shift_power_a: usize,
    clock_power_b: usize,
) -> QuditLabReport {
    let d = dimension_d.clamp(2, 8);
    let fd = qudit_qft(d);
    let xd = qudit_shift_x(d);
    let zd = qudit_clock_z(d);

    let reg = match preset {
        QuditPreset::SingleFourierSuperposition => {
            let mut r = QuditRegister::zero_state(d, 1);
            for _ in 0..(shift_power_a % d) {
                r.apply_single(0, &xd);
            }
            r.apply_single(0, &fd);
            r
        }
        QuditPreset::WeylClockShiftOrbit => {
            let mut r = QuditRegister::zero_state(d, 1);
            r.apply_single(0, &fd);
            for _ in 0..(clock_power_b % d) {
                r.apply_single(0, &zd);
            }
            for _ in 0..(shift_power_a % d) {
                r.apply_single(0, &xd);
            }
            r
        }
        QuditPreset::BellMaximallyEntangledPair => {
            let mut r = QuditRegister::zero_state(d, 2);
            r.apply_single(0, &fd);
            r.apply_csum(0, 1);
            // Optional local Weyl displacement on qudit 0 to sweep generalized Bell basis states |Phi_{a,b}>
            for _ in 0..(clock_power_b % d) {
                r.apply_single(0, &zd);
            }
            for _ in 0..(shift_power_a % d) {
                r.apply_single(1, &xd);
            }
            r
        }
        QuditPreset::ThreeQuditGhzRing => {
            // Cap d <= 5 on 3 qudits to keep basis visualization crisp (d^3 <= 125)
            let d_ghz = d.min(5);
            let fd_g = qudit_qft(d_ghz);
            let mut r = QuditRegister::zero_state(d_ghz, 3);
            r.apply_single(0, &fd_g);
            r.apply_csum(0, 1);
            r.apply_csum(0, 2);
            r
        }
    };

    let eff_d = reg.dimension_d;
    let basis_labels: Vec<String> = (0..reg.amps.len()).map(|i| reg.basis_label(i)).collect();
    let probabilities: Vec<f64> = reg.amps.iter().map(|a| a.norm_sqr()).collect();
    let phases_rad: Vec<f64> = reg.amps.iter().map(|a| a.arg()).collect();

    let mut qkd_capacity_curve = Vec::with_capacity(61);
    let mut tolerated_qber_pct = 0.0;
    for step in 0..=60 {
        let q = (step as f64) * 0.005; // 0.0% to 30.0%
        let r2 = qudit_qkd_secret_bits_per_sifted(2, q);
        let rd = qudit_qkd_secret_bits_per_sifted(eff_d, q);
        if rd > 0.0 {
            tolerated_qber_pct = q * 100.0;
        }
        qkd_capacity_curve.push((q * 100.0, r2, rd));
    }

    QuditLabReport {
        dimension_d: eff_d,
        preset,
        shift_power_a: shift_power_a % eff_d,
        clock_power_b: clock_power_b % eff_d,
        basis_labels,
        probabilities,
        phases_rad,
        entanglement_ebits: reg.entanglement_ebits_qudit0(),
        max_ebits_for_d: (eff_d as f64).log2(),
        weyl_residual: weyl_commutation_residual(eff_d),
        qft_is_unitary: fd.is_unitary(1e-10),
        qkd_capacity_curve,
        tolerated_qber_pct,
    }
}
