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

//! Pure safe-Rust complex arithmetic, dense unitary/Hermitian matrix operations,
//! Kronecker tensor products, partial traces, and Jacobi Hermitian eigensolver.

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use std::fmt;
use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Complex64 {
    pub re: f64,
    pub im: f64,
}

impl Complex64 {
    pub const ZERO: Self = Self { re: 0.0, im: 0.0 };
    pub const ONE: Self = Self { re: 1.0, im: 0.0 };
    pub const I: Self = Self { re: 0.0, im: 1.0 };

    #[inline]
    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    #[inline]
    pub fn from_polar(r: f64, theta: f64) -> Self {
        Self {
            re: r * theta.cos(),
            im: r * theta.sin(),
        }
    }

    #[inline]
    pub fn cis(theta: f64) -> Self {
        Self::from_polar(1.0, theta)
    }

    #[inline]
    pub fn conj(self) -> Self {
        Self {
            re: self.re,
            im: -self.im,
        }
    }

    #[inline]
    pub fn norm_sqr(self) -> f64 {
        self.re * self.re + self.im * self.im
    }

    #[inline]
    pub fn norm(self) -> f64 {
        self.norm_sqr().sqrt()
    }

    #[inline]
    pub fn arg(self) -> f64 {
        self.im.atan2(self.re)
    }

    #[inline]
    pub fn scale(self, s: f64) -> Self {
        Self {
            re: self.re * s,
            im: self.im * s,
        }
    }

    pub fn approx_eq(self, other: Self, tol: f64) -> bool {
        (self - other).norm() <= tol
    }
}

impl fmt::Display for Complex64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.im.abs() < 1e-10 {
            write!(f, "{:.4}", self.re)
        } else if self.re.abs() < 1e-10 {
            write!(f, "{:.4}i", self.im)
        } else if self.im >= 0.0 {
            write!(f, "{:.4} + {:.4}i", self.re, self.im)
        } else {
            write!(f, "{:.4} - {:.4}i", self.re, -self.im)
        }
    }
}

impl Add for Complex64 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            re: self.re + rhs.re,
            im: self.im + rhs.im,
        }
    }
}

impl AddAssign for Complex64 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.re += rhs.re;
        self.im += rhs.im;
    }
}

impl Sub for Complex64 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            re: self.re - rhs.re,
            im: self.im - rhs.im,
        }
    }
}

impl SubAssign for Complex64 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.re -= rhs.re;
        self.im -= rhs.im;
    }
}

impl Neg for Complex64 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self::Output {
        Self {
            re: -self.re,
            im: -self.im,
        }
    }
}

impl Mul for Complex64 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            re: self.re * rhs.re - self.im * rhs.im,
            im: self.re * rhs.im + self.im * rhs.re,
        }
    }
}

impl MulAssign for Complex64 {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Mul<f64> for Complex64 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f64) -> Self::Output {
        self.scale(rhs)
    }
}

/// Square complex matrix of dimension `dim x dim` stored in row-major order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CMatrix {
    pub dim: usize,
    pub data: Vec<Complex64>,
}

impl CMatrix {
    pub fn zeros(dim: usize) -> Self {
        Self {
            dim,
            data: vec![Complex64::ZERO; dim * dim],
        }
    }

    pub fn identity(dim: usize) -> Self {
        let mut m = Self::zeros(dim);
        for i in 0..dim {
            m.set(i, i, Complex64::ONE);
        }
        m
    }

    pub fn from_rows(dim: usize, data: Vec<Complex64>) -> Self {
        assert_eq!(data.len(), dim * dim, "CMatrix dimension mismatch");
        Self { dim, data }
    }

    #[inline]
    pub fn get(&self, r: usize, c: usize) -> Complex64 {
        self.data[r * self.dim + c]
    }

    #[inline]
    pub fn set(&mut self, r: usize, c: usize, val: Complex64) {
        self.data[r * self.dim + c] = val;
    }

    #[inline]
    pub fn add_at(&mut self, r: usize, c: usize, val: Complex64) {
        self.data[r * self.dim + c] += val;
    }

    pub fn dagger(&self) -> Self {
        let mut out = Self::zeros(self.dim);
        for r in 0..self.dim {
            for c in 0..self.dim {
                out.set(c, r, self.get(r, c).conj());
            }
        }
        out
    }

    pub fn scale(&self, factor: Complex64) -> Self {
        Self {
            dim: self.dim,
            data: self.data.iter().map(|&x| x * factor).collect(),
        }
    }

    pub fn add_mat(&self, rhs: &Self) -> Self {
        assert_eq!(self.dim, rhs.dim);
        Self {
            dim: self.dim,
            data: self
                .data
                .iter()
                .zip(rhs.data.iter())
                .map(|(&a, &b)| a + b)
                .collect(),
        }
    }

    pub fn matmul(&self, rhs: &Self) -> Self {
        assert_eq!(self.dim, rhs.dim);
        let n = self.dim;
        let mut out = Self::zeros(n);
        for i in 0..n {
            for k in 0..n {
                let aik = self.get(i, k);
                if aik.norm_sqr() == 0.0 {
                    continue;
                }
                for j in 0..n {
                    let prev = out.get(i, j);
                    out.set(i, j, prev + aik * rhs.get(k, j));
                }
            }
        }
        out
    }

    pub fn apply_vec(&self, v: &[Complex64]) -> Vec<Complex64> {
        assert_eq!(self.dim, v.len());
        let mut out = vec![Complex64::ZERO; self.dim];
        for r in 0..self.dim {
            let mut acc = Complex64::ZERO;
            for c in 0..self.dim {
                acc += self.get(r, c) * v[c];
            }
            out[r] = acc;
        }
        out
    }

    pub fn kron(&self, rhs: &Self) -> Self {
        let d1 = self.dim;
        let d2 = rhs.dim;
        let mut out = Self::zeros(d1 * d2);
        for r1 in 0..d1 {
            for c1 in 0..d1 {
                let a = self.get(r1, c1);
                if a.norm_sqr() == 0.0 {
                    continue;
                }
                for r2 in 0..d2 {
                    for c2 in 0..d2 {
                        out.set(r1 * d2 + r2, c1 * d2 + c2, a * rhs.get(r2, c2));
                    }
                }
            }
        }
        out
    }

    pub fn trace(&self) -> Complex64 {
        let mut acc = Complex64::ZERO;
        for i in 0..self.dim {
            acc += self.get(i, i);
        }
        acc
    }

    pub fn purity(&self) -> f64 {
        // For Hermitian density matrix rho, Tr(rho^2) = sum_{i,j} |rho_{ij}|^2
        self.data.iter().map(|z| z.norm_sqr()).sum()
    }

    pub fn is_unitary(&self, tol: f64) -> bool {
        let prod = self.dagger().matmul(self);
        let id = Self::identity(self.dim);
        prod.data
            .iter()
            .zip(id.data.iter())
            .all(|(a, b)| a.approx_eq(*b, tol))
    }

    /// Computes eigenvalues of a Hermitian matrix via real block embedding + Jacobi sweeps.
    /// A Hermitian `n x n` matrix H = A + i B maps to real symmetric `2n x 2n` matrix
    /// [[A, -B], [B, A]] whose eigenvalues appear in identical pairs!
    pub fn hermitian_eigenvalues(&self) -> Vec<f64> {
        let n = self.dim;
        if n == 0 {
            return Vec::new();
        }
        if n == 1 {
            return vec![self.get(0, 0).re];
        }
        let m = 2 * n;
        let mut sym = vec![0.0_f64; m * m];
        for r in 0..n {
            for c in 0..n {
                let z = self.get(r, c);
                sym[r * m + c] = z.re;
                sym[r * m + (c + n)] = -z.im;
                sym[(r + n) * m + c] = z.im;
                sym[(r + n) * m + (c + n)] = z.re;
            }
        }
        // Symmetrize numerical round-off
        for r in 0..m {
            for c in (r + 1)..m {
                let avg = 0.5 * (sym[r * m + c] + sym[c * m + r]);
                sym[r * m + c] = avg;
                sym[c * m + r] = avg;
            }
        }

        // Classical Jacobi rotation sweeps
        let max_sweeps = 64;
        for _sweep in 0..max_sweeps {
            let mut max_off = 0.0_f64;
            for p in 0..m {
                for q in (p + 1)..m {
                    max_off = max_off.max(sym[p * m + q].abs());
                }
            }
            if max_off < 1e-12 {
                break;
            }
            for p in 0..m {
                for q in (p + 1)..m {
                    let apq = sym[p * m + q];
                    if apq.abs() < 1e-14 {
                        continue;
                    }
                    let app = sym[p * m + p];
                    let aqq = sym[q * m + q];
                    let tau = (aqq - app) / (2.0 * apq);
                    let t = if tau >= 0.0 {
                        1.0 / (tau + (1.0 + tau * tau).sqrt())
                    } else {
                        -1.0 / (-tau + (1.0 + tau * tau).sqrt())
                    };
                    let cos_t = 1.0 / (1.0 + t * t).sqrt();
                    let sin_t = t * cos_t;

                    sym[p * m + p] = app - t * apq;
                    sym[q * m + q] = aqq + t * apq;
                    sym[p * m + q] = 0.0;
                    sym[q * m + p] = 0.0;

                    for k in 0..m {
                        if k != p && k != q {
                            let akp = sym[k * m + p];
                            let akq = sym[k * m + q];
                            let new_kp = cos_t * akp - sin_t * akq;
                            let new_kq = sin_t * akp + cos_t * akq;
                            sym[k * m + p] = new_kp;
                            sym[p * m + k] = new_kp;
                            sym[k * m + q] = new_kq;
                            sym[q * m + k] = new_kq;
                        }
                    }
                }
            }
        }

        let mut eigs: Vec<f64> = (0..m).map(|i| sym[i * m + i]).collect();
        eigs.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        // Take every second eigenvalue from the doubled real-block spectrum
        (0..n).map(|i| 0.5 * (eigs[2 * i] + eigs[2 * i + 1])).collect()
    }

    /// Computes von Neumann entropy S(rho) = - sum_i lambda_i log2(lambda_i) in bits (ebits).
    pub fn von_neumann_entropy_bits(&self) -> f64 {
        let eigs = self.hermitian_eigenvalues();
        let mut s = 0.0;
        for lam in eigs {
            if lam > 1e-12 {
                s -= lam * lam.log2();
            }
        }
        s.max(0.0)
    }
}

/// Binary Shannon entropy h2(p) = -p log2(p) - (1-p) log2(1-p).
pub fn binary_entropy(p: f64) -> f64 {
    let pc = p.clamp(0.0, 1.0);
    if pc <= 1e-15 || pc >= 1.0 - 1e-15 {
        return 0.0;
    }
    -pc * pc.log2() - (1.0 - pc) * (1.0 - pc).log2()
}

/// Converts a single-qubit 2x2 density matrix into its Bloch vector (rx, ry, rz).
pub fn rho_to_bloch(rho: &CMatrix) -> [f64; 3] {
    assert_eq!(rho.dim, 2);
    let rho00 = rho.get(0, 0).re;
    let rho11 = rho.get(1, 1).re;
    let rho01 = rho.get(0, 1);
    let rx = 2.0 * rho01.re;
    let ry = -2.0 * rho01.im;
    let rz = rho00 - rho11;
    [rx, ry, rz]
}

/// Maps phase angle in [-pi, pi] to RGB [u8; 3] on a smooth cyclic color wheel.
pub fn phase_to_rgb(phase_rad: f64) -> [u8; 3] {
    let normalized = ((phase_rad + PI) / (2.0 * PI)).rem_euclid(1.0);
    let h = normalized * 6.0;
    let s = 0.78_f64;
    let v = 0.92_f64;
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let m = v - c;
    let (r1, g1, b1) = if h < 1.0 {
        (c, x, 0.0)
    } else if h < 2.0 {
        (x, c, 0.0)
    } else if h < 3.0 {
        (0.0, c, x)
    } else if h < 4.0 {
        (0.0, x, c)
    } else if h < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    [
        ((r1 + m) * 255.0).round() as u8,
        ((g1 + m) * 255.0).round() as u8,
        ((b1 + m) * 255.0).round() as u8,
    ]
}
