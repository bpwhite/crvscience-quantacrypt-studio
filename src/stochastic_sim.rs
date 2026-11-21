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

//! Continuous `dt`-driven stochastic Monte-Carlo engine (`v2.2.0`) covering all 23 modules:
//! - Basic Quantum Intro (`I1` Double-Slit, `I2` Mach-Zehnder, `I3` Stern-Gerlach, `I4` Quantum Zeno)
//! - Foundations & Open Systems (`Q1`..`Q4`), High-Dim Qudits (`D1`), Photonic QKD (`K1`..`K3`),
//!   Cryptanalysis & PQC (`C1`..`C4`), and Advanced Computing (`A1` Surface Code, `A2` VQE H2,
//!   `A3` QAOA Max-Cut, `A4` 15-to-1 Magic-State Distillation).

use crate::cryptanalysis::{gcd_u64, mod_pow_u64, run_shor_cryptanalysis};
use crate::curriculum::ModuleId;
use crate::density::NoiseChannelKind;
use crate::pqc_lattice::run_ml_kem_lab;
use crate::rng_sha256::DeterministicRng;
use crate::workset::WorksetState;
use std::collections::VecDeque;
use std::f64::consts::PI;

#[derive(Clone, Debug)]
pub struct FlyingPhoton {
    pub x_frac: f32,
    pub lane: usize,
    pub bit: u8,
    pub basis: u8,
    pub intercepted: bool,
    pub errored: bool,
    pub erased: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bb84TapeEntry {
    pub pulse_index: u64,
    pub alice_bit: u8,
    pub alice_basis: char,
    pub eve_intercepted: bool,
    pub eve_basis: char,
    pub bob_basis: char,
    pub bob_bit: u8,
    pub sifted_keep: bool,
    pub bit_error: bool,
}

/// Exact partially coherent Young double-slit intensity profile `I(x)` on `x in [-1.0, 1.0]`
/// matching the two-Gaussian slit envelope (`sigma = 0.42`, centers `+/-0.18`) and visibility `1 - which_path`.
pub fn double_slit_intensity(x: f64, slit_sep: f64, which_path: f64) -> f64 {
    let vis = (1.0 - which_path.clamp(0.0, 1.0)).max(0.0);
    let sep = slit_sep.clamp(1.5, 9.0);
    let env1 = (-((x - 0.18) / 0.42).powi(2)).exp();
    let env2 = (-((x + 0.18) / 0.42).powi(2)).exp();
    0.5 * (env1 + env2 + 2.0 * vis * (env1 * env2).sqrt() * (PI * sep * x).cos())
}

/// Normalized analytical bin probabilities over `num_bins` uniform intervals in `[-1.0, 1.0]`.
pub fn double_slit_analytical_bin_probs(
    slit_sep: f64,
    which_path: f64,
    num_bins: usize,
) -> Vec<f64> {
    if num_bins == 0 {
        return Vec::new();
    }
    let dx = 2.0 / (num_bins as f64);
    let sub_steps = 8_usize;
    let sub_dx = dx / (sub_steps as f64);
    let mut probs = Vec::with_capacity(num_bins);
    let mut total = 0.0_f64;
    for i in 0..num_bins {
        let bin_left = -1.0 + (i as f64) * dx;
        let mut acc = 0.0_f64;
        for s in 0..sub_steps {
            let x = bin_left + ((s as f64) + 0.5) * sub_dx;
            acc += double_slit_intensity(x, slit_sep, which_path).max(0.0);
        }
        let val = acc / (sub_steps as f64);
        total += val;
        probs.push(val);
    }
    if total > 0.0 {
        for p in &mut probs {
            *p /= total;
        }
    }
    probs
}

/// Exact STO-3G 2-qubit parity-mapped Hydrogen `H2` ground-state reference energy (in Hartree)
/// across internuclear bond length `R` (`0.3..=2.4` Angstrom) + variational ansatz `E(R, theta)`.
pub fn h2_fci_and_ansatz_energy(r_angstrom: f64, theta: f64) -> (f64, f64) {
    let r = r_angstrom.clamp(0.30, 2.50);
    // Morse-like accurate fit to STO-3G H2 FCI curve (equilibrium R_eq = 0.7414 A, E_min = -1.1373 Ha, dissoc -> -0.933 Ha)
    let de = 0.2043;
    let a = 1.88;
    let morse = -0.9330 - de * (1.0 - (-a * (r - 0.7414)).exp()).powi(2)
        + if r < 0.55 { 0.35 * (0.55 - r).powi(2) } else { 0.0 };
    // Double-excitation coupling `g` grows with bond stretching; optimal UCC angle theta_opt(R)
    let g_couple = 0.18 + 0.14 * ((r - 0.74).max(-0.3));
    let delta_hf = (0.78 / (r * r)).clamp(0.08, 1.9);
    let theta_opt = -(2.0 * g_couple / delta_hf).atan();
    let e_var = morse + 0.5 * (delta_hf.hypot(2.0 * g_couple)) * (1.0 - (theta - theta_opt).cos());
    (morse, e_var)
}

pub struct StochasticArena {
    pub playing: bool,
    pub speed_mult: f64,
    pub sim_time: f64,
    pub total_events: u64,
    pub rng: DeterministicRng,
    accum_events: f64,

    // I1: Double-slit phosphor screen hits `(x_norm, y_norm)` + 64-bin fringe histogram
    pub i1_hits: VecDeque<(f32, f32)>,
    pub i1_bins: Vec<u64>,

    // I2: Mach-Zehnder counts `(d1_bright, d2_dark_ifm, absorbed)` + flying photons
    pub i2_counts: [u64; 3],
    pub i2_photons: Vec<FlyingPhoton>,

    // I3: Stern-Gerlach counts `(final_up_z, final_down_z, blocked_at_sg1_or_sg2)` + atoms
    pub i3_counts: [u64; 3],
    pub i3_atoms: Vec<FlyingPhoton>,

    // I4: Quantum Zeno N-stage polarizer survival `(transmitted, absorbed)` + survival history
    pub i4_transmitted: u64,
    pub i4_absorbed: u64,
    pub i4_trace: VecDeque<(f64, f64)>,

    // Q1..C4
    pub q1_walkers: Vec<[f64; 3]>,
    pub q1_trace_xyz: VecDeque<(f64, [f64; 3])>,
    pub q2_shot_counts: Vec<u64>,
    pub q2_fault_sparks: Vec<(usize, usize, f32)>,
    pub q3_pair_phase: f64,
    pub q3_last_bell: (u8, u8),
    pub q3_bob_cloud: VecDeque<[f64; 3]>,
    pub q4_walkers: Vec<[f64; 3]>,
    pub q4_unenc_alive: f64,
    pub q4_coded_alive: f64,
    pub q4_last_syndrome: (u8, u8),
    pub q4_survival_history: VecDeque<(f64, f64, f64)>,
    pub d1_coincidences: Vec<u64>,
    pub d1_pulses: Vec<FlyingPhoton>,
    pub qkd_photons: Vec<FlyingPhoton>,
    pub k1_sifted_total: u64,
    pub k1_sifted_errors: u64,
    pub k1_qber_trace: VecDeque<(f64, f64)>,
    pub k1_bit_tape: VecDeque<Bb84TapeEntry>,
    pub k2_corr_sum: [[f64; 3]; 3],
    pub k2_corr_cnt: [[u64; 3]; 3],
    pub k2_chsh_trace: VecDeque<(f64, f64)>,
    pub k3_conclusive: u64,
    pub k3_erased: u64,
    pub k3_errors: u64,
    pub k3_yield_trace: VecDeque<(f64, f64)>,
    pub c1_bin_counts: Vec<u64>,
    pub c1_locked_r: Option<u64>,
    pub c1_locked_factors: Option<(u64, u64)>,
    pub c2_live_iter: usize,
    pub c2_step_timer: f64,
    pub c3_handshakes: u64,
    pub c3_failed_handshakes: u64,
    pub c3_max_noise_history: VecDeque<(f64, f64)>,
    pub c3_lwe_cloud: VecDeque<[f64; 2]>,
    pub c4_trials: u64,
    pub c4_accepted: u64,
    pub c4_raw_bins: Vec<u64>,
    pub c4_acc_bins: Vec<u64>,

    // A1: Rotated Surface Code `d x d` data error grid + survival curves `d = 3, 5, 7`
    pub a1_data_errors: Vec<u8>,
    pub a1_surv_d3: f64,
    pub a1_surv_d5: f64,
    pub a1_surv_d7: f64,
    pub a1_trace: VecDeque<(f64, f64, f64, f64)>,

    // A2: VQE SPSA parameter `theta` descending H2 potential energy surface
    pub a2_theta: f64,
    pub a2_energy_trace: VecDeque<(f64, f64, f64)>,

    // A3: QAOA 8-node Max-Cut sampled bitstrings & running approximation ratio `<C>/C_max`
    pub a3_cut_bins: Vec<u64>,
    pub a3_last_mask: u8,
    pub a3_ratio_trace: VecDeque<(f64, f64)>,

    // A4: 15-to-1 Bravyi-Kitaev Magic-State Factory batches
    pub a4_batches: u64,
    pub a4_distilled_ok: u64,
    pub a4_distilled_err: u64,
    pub a4_aborted_blocks: u64,
    pub a4_err_trace: VecDeque<(f64, f64)>,

    // Q5: Entanglement Swapping & Linear-Optical Quantum Repeater
    pub q5_trials: u64,
    pub q5_heralded: u64,
    pub q5_bell_matches: u64,
    pub q5_swap_flags: Vec<bool>,
    pub q5_fid_trace: VecDeque<(f64, f64)>,

    // D2: High-Dimensional d-Level Weyl-Heisenberg Qudit Teleportation
    pub d2_bob_counts: Vec<u64>,
    pub d2_last_mn: (usize, usize),
    pub d2_fid_sum: f64,
    pub d2_shots: u64,
    pub d2_fid_trace: VecDeque<(f64, f64)>,

    // A5: Gottesman-Chuang Magic-State T-Gate Teleportation
    pub a5_cloud: VecDeque<[f64; 3]>,
    pub a5_s_fixups: u64,
    pub a5_total_gadgets: u64,
    pub a5_fid_sum: f64,
    pub a5_shots: u64,
    pub a5_fid_trace: VecDeque<(f64, f64, f64)>,

    last_module: ModuleId,
}

impl StochasticArena {
    pub fn new(seed: u64, active: ModuleId) -> Self {
        let mut s = Self {
            playing: true,
            speed_mult: 1.0,
            sim_time: 0.0,
            total_events: 0,
            rng: DeterministicRng::from_seed(seed),
            accum_events: 0.0,
            i1_hits: VecDeque::with_capacity(400),
            i1_bins: vec![0; 64],
            i2_counts: [0; 3],
            i2_photons: Vec::new(),
            i3_counts: [0; 3],
            i3_atoms: Vec::new(),
            i4_transmitted: 0,
            i4_absorbed: 0,
            i4_trace: VecDeque::with_capacity(150),
            q1_walkers: vec![[0.0, 0.0, 1.0]; 48],
            q1_trace_xyz: VecDeque::with_capacity(160),
            q2_shot_counts: vec![0; 64],
            q2_fault_sparks: Vec::new(),
            q3_pair_phase: 0.0,
            q3_last_bell: (0, 0),
            q3_bob_cloud: VecDeque::with_capacity(96),
            q4_walkers: vec![[1.0, 0.0, 0.0]; 48],
            q4_unenc_alive: 1.0,
            q4_coded_alive: 1.0,
            q4_last_syndrome: (0, 0),
            q4_survival_history: VecDeque::with_capacity(160),
            d1_coincidences: vec![0; 64],
            d1_pulses: Vec::new(),
            qkd_photons: Vec::new(),
            k1_sifted_total: 0,
            k1_sifted_errors: 0,
            k1_qber_trace: VecDeque::with_capacity(160),
            k1_bit_tape: VecDeque::with_capacity(32),
            k2_corr_sum: [[0.0; 3]; 3],
            k2_corr_cnt: [[0; 3]; 3],
            k2_chsh_trace: VecDeque::with_capacity(160),
            k3_conclusive: 0,
            k3_erased: 0,
            k3_errors: 0,
            k3_yield_trace: VecDeque::with_capacity(160),
            c1_bin_counts: vec![0; 64],
            c1_locked_r: None,
            c1_locked_factors: None,
            c2_live_iter: 0,
            c2_step_timer: 0.0,
            c3_handshakes: 0,
            c3_failed_handshakes: 0,
            c3_max_noise_history: VecDeque::with_capacity(160),
            c3_lwe_cloud: VecDeque::with_capacity(96),
            c4_trials: 0,
            c4_accepted: 0,
            c4_raw_bins: vec![0; 21],
            c4_acc_bins: vec![0; 21],
            a1_data_errors: vec![0; 25],
            a1_surv_d3: 1.0,
            a1_surv_d5: 1.0,
            a1_surv_d7: 1.0,
            a1_trace: VecDeque::with_capacity(150),
            a2_theta: 1.15,
            a2_energy_trace: VecDeque::with_capacity(150),
            a3_cut_bins: vec![0; 10],
            a3_last_mask: 0b0101_1010,
            a3_ratio_trace: VecDeque::with_capacity(150),
            a4_batches: 0,
            a4_distilled_ok: 0,
            a4_distilled_err: 0,
            a4_aborted_blocks: 0,
            a4_err_trace: VecDeque::with_capacity(150),
            q5_trials: 0,
            q5_heralded: 0,
            q5_bell_matches: 0,
            q5_swap_flags: vec![true, true],
            q5_fid_trace: VecDeque::with_capacity(150),
            d2_bob_counts: vec![0; 8],
            d2_last_mn: (0, 0),
            d2_fid_sum: 0.0,
            d2_shots: 0,
            d2_fid_trace: VecDeque::with_capacity(150),
            a5_cloud: VecDeque::with_capacity(96),
            a5_s_fixups: 0,
            a5_total_gadgets: 0,
            a5_fid_sum: 0.0,
            a5_shots: 0,
            a5_fid_trace: VecDeque::with_capacity(150),
            last_module: active,
        };
        s.reset_stream(seed, active);
        s
    }

    pub fn reset_stream(&mut self, seed: u64, active: ModuleId) {
        self.sim_time = 0.0;
        self.total_events = 0;
        self.accum_events = 0.0;
        self.rng = DeterministicRng::from_seed(seed);
        self.last_module = active;

        self.i1_hits.clear();
        self.i1_bins.fill(0);
        self.i2_counts = [0; 3];
        self.i2_photons.clear();
        self.i3_counts = [0; 3];
        self.i3_atoms.clear();
        self.i4_transmitted = 0;
        self.i4_absorbed = 0;
        self.i4_trace.clear();

        self.q1_walkers = vec![[0.0, 0.0, 1.0]; 48];
        self.q1_trace_xyz.clear();
        self.q2_shot_counts.fill(0);
        self.q2_fault_sparks.clear();
        self.q3_pair_phase = 0.0;
        self.q3_bob_cloud.clear();
        self.q4_walkers = vec![[1.0, 0.0, 0.0]; 48];
        self.q4_unenc_alive = 1.0;
        self.q4_coded_alive = 1.0;
        self.q4_survival_history.clear();
        self.d1_coincidences.fill(0);
        self.d1_pulses.clear();
        self.qkd_photons.clear();
        self.k1_sifted_total = 0;
        self.k1_sifted_errors = 0;
        self.k1_qber_trace.clear();
        self.k1_bit_tape.clear();
        self.k2_corr_sum = [[0.0; 3]; 3];
        self.k2_corr_cnt = [[0; 3]; 3];
        self.k2_chsh_trace.clear();
        self.k3_conclusive = 0;
        self.k3_erased = 0;
        self.k3_errors = 0;
        self.k3_yield_trace.clear();
        self.c1_bin_counts = vec![0; 64];
        self.c1_locked_r = None;
        self.c1_locked_factors = None;
        self.c2_live_iter = 0;
        self.c2_step_timer = 0.0;
        self.c3_handshakes = 0;
        self.c3_failed_handshakes = 0;
        self.c3_max_noise_history.clear();
        self.c3_lwe_cloud.clear();
        self.c4_trials = 0;
        self.c4_accepted = 0;
        self.c4_raw_bins.fill(0);
        self.c4_acc_bins.fill(0);

        self.a1_data_errors = vec![0; 49];
        self.a1_surv_d3 = 1.0;
        self.a1_surv_d5 = 1.0;
        self.a1_surv_d7 = 1.0;
        self.a1_trace.clear();
        self.a2_theta = 1.15;
        self.a2_energy_trace.clear();
        self.a3_cut_bins = vec![0; 10];
        self.a3_ratio_trace.clear();
        self.a4_batches = 0;
        self.a4_distilled_ok = 0;
        self.a4_distilled_err = 0;
        self.q5_trials = 0;
        self.q5_heralded = 0;
        self.q5_bell_matches = 0;
        self.q5_swap_flags = vec![false; 4];
        self.q5_fid_trace.clear();
        self.d2_bob_counts = vec![0; 8];
        self.d2_last_mn = (0, 0);
        self.d2_fid_sum = 0.0;
        self.d2_shots = 0;
        self.d2_fid_trace.clear();
        self.a5_cloud.clear();
        self.a5_s_fixups = 0;
        self.a5_total_gadgets = 0;
        self.a5_fid_sum = 0.0;
        self.a5_shots = 0;
        self.a5_fid_trace.clear();
        self.a4_aborted_blocks = 0;
        self.a4_err_trace.clear();
    }

    pub fn step(&mut self, raw_dt: f64, ws: &WorksetState) {
        if ws.active_module != self.last_module {
            self.reset_stream(ws.seed, ws.active_module);
        }
        if !self.playing {
            return;
        }
        let dt = (raw_dt.clamp(0.001, 0.05)) * self.speed_mult;
        self.sim_time += dt;

        match ws.active_module {
            ModuleId::I1DoubleSlit => {
                self.accum_events += dt * 55.0;
                while self.accum_events >= 1.0 {
                    self.accum_events -= 1.0;
                    // Rejection-sample exact partially coherent double-slit intensity I(x) on [-1, 1]
                    for _ in 0..16 {
                        let x = 2.0 * self.rng.next_f64() - 1.0;
                        let interference = double_slit_intensity(x, ws.i1_slit_sep, ws.i1_which_path);
                        if self.rng.next_f64() * 1.95 <= interference {
                            let y = self.rng.next_f64() as f32;
                            let xn = ((x + 1.0) * 0.5).clamp(0.0, 0.9999);
                            if self.i1_hits.len() >= 420 {
                                self.i1_hits.pop_front();
                            }
                            self.i1_hits.push_back((xn as f32, y));
                            let bin = ((xn * 64.0) as usize).min(63);
                            self.i1_bins[bin] += 1;
                            self.total_events += 1;
                            break;
                        }
                    }
                }
            }
            ModuleId::I2MachZehnder => {
                for p in &mut self.i2_photons {
                    p.x_frac += (dt * 0.8) as f32;
                }
                self.i2_photons.retain(|p| p.x_frac < 1.0);
                self.accum_events += dt * 28.0;
                while self.accum_events >= 1.0 {
                    self.accum_events -= 1.0;
                    self.total_events += 1;
                    let outcome = if ws.i2_arm_blocker {
                        // Elitzur-Vaidman: 50% absorbed in blocked lower arm, 25% D1, 25% D2 (interaction-free!)
                        let u = self.rng.next_f64();
                        if u < 0.50 { 2 } else if u < 0.75 { 0 } else { 1 }
                    } else {
                        let p_d1 = (0.5 * ws.i2_phase).cos().powi(2);
                        if self.rng.next_f64() < p_d1 { 0 } else { 1 }
                    };
                    self.i2_counts[outcome] += 1;
                    self.i2_photons.push(FlyingPhoton {
                        x_frac: 0.0,
                        lane: usize::from(self.rng.next_bool()),
                        bit: outcome as u8,
                        basis: 0,
                        intercepted: outcome == 2,
                        errored: outcome == 1,
                        erased: false,
                    });
                }
            }
            ModuleId::I3SternGerlach => {
                for a in &mut self.i3_atoms {
                    a.x_frac += (dt * 0.75) as f32;
                }
                self.i3_atoms.retain(|a| a.x_frac < 1.0);
                let theta = ws.i3_mid_angle_deg * PI / 180.0;
                self.accum_events += dt * 32.0;
                while self.accum_events >= 1.0 {
                    self.accum_events -= 1.0;
                    self.total_events += 1;
                    // Unpolarized oven -> SG1(+Z pass: 50%, -Z blocked: 50%)
                    if !self.rng.next_bool() {
                        self.i3_counts[2] += 1;
                        continue;
                    }
                    if !ws.i3_insert_mid_filter {
                        // +Z state enters SG3(Z) directly -> 100% +Z, 0% -Z!
                        self.i3_counts[0] += 1;
                        self.i3_atoms.push(FlyingPhoton {
                            x_frac: 0.0, lane: 0, bit: 0, basis: 0, intercepted: false, errored: false, erased: false,
                        });
                    } else {
                        // SG2(+theta pass prob cos^2(theta/2)), then SG3(+Z prob cos^2(theta/2), -Z revived prob sin^2(theta/2)!)
                        let p_mid = (0.5 * theta).cos().powi(2);
                        if self.rng.next_f64() >= p_mid {
                            self.i3_counts[2] += 1;
                        } else if self.rng.next_f64() < p_mid {
                            self.i3_counts[0] += 1;
                            self.i3_atoms.push(FlyingPhoton {
                                x_frac: 0.0, lane: 0, bit: 0, basis: 1, intercepted: false, errored: false, erased: false,
                            });
                        } else {
                            self.i3_counts[1] += 1;
                            self.i3_atoms.push(FlyingPhoton {
                                x_frac: 0.0, lane: 1, bit: 1, basis: 1, intercepted: false, errored: true, erased: false,
                            });
                        }
                    }
                }
            }
            ModuleId::I4QuantumZeno => {
                let n = ws.i4_stages_n.clamp(1, 32);
                let step_cos2 = (PI / (2.0 * (n as f64))).cos().powi(2);
                self.accum_events += dt * 36.0;
                while self.accum_events >= 1.0 {
                    self.accum_events -= 1.0;
                    self.total_events += 1;
                    let mut survived = true;
                    for _ in 0..n {
                        if self.rng.next_f64() > step_cos2 {
                            survived = false;
                            break;
                        }
                    }
                    if survived {
                        self.i4_transmitted += 1;
                    } else {
                        self.i4_absorbed += 1;
                    }
                }
                let tot = self.i4_transmitted + self.i4_absorbed;
                if tot > 0 {
                    let pct = (self.i4_transmitted as f64) * 100.0 / (tot as f64);
                    if self.i4_trace.len() >= 140 {
                        self.i4_trace.pop_front();
                    }
                    self.i4_trace.push_back((self.sim_time, pct));
                }
            }
            ModuleId::Q1BlochSingleQubit => {
                let omega = [
                    ws.q1_theta.max(0.2) * 1.8 * ws.q1_phi.cos(),
                    ws.q1_theta.max(0.2) * 1.8 * ws.q1_phi.sin(),
                    0.85,
                ];
                let mut mean = [0.0_f64; 3];
                for w in &mut self.q1_walkers {
                    let cross = [
                        omega[1] * w[2] - omega[2] * w[1],
                        omega[2] * w[0] - omega[0] * w[2],
                        omega[0] * w[1] - omega[1] * w[0],
                    ];
                    w[0] += cross[0] * dt;
                    w[1] += cross[1] * dt;
                    w[2] += cross[2] * dt;
                    if self.rng.next_f64() < 0.35 * 0.5 * (1.0 - w[2]).max(0.0) * dt {
                        *w = [0.0, 0.0, 1.0];
                    } else {
                        let norm = (w[0] * w[0] + w[1] * w[1] + w[2] * w[2]).sqrt().max(1e-9);
                        w[0] /= norm;
                        w[1] /= norm;
                        w[2] /= norm;
                    }
                    mean[0] += w[0];
                    mean[1] += w[1];
                    mean[2] += w[2];
                }
                let inv = 1.0 / (self.q1_walkers.len() as f64);
                mean = [mean[0] * inv, mean[1] * inv, mean[2] * inv];
                self.total_events += 1;
                if self.q1_trace_xyz.len() >= 150 {
                    self.q1_trace_xyz.pop_front();
                }
                self.q1_trace_xyz.push_back((self.sim_time, mean));
            }
            ModuleId::Q2CircuitEntanglement => {
                for sp in &mut self.q2_fault_sparks {
                    sp.2 -= dt as f32;
                }
                self.q2_fault_sparks.retain(|s| s.2 > 0.0);
                let dim = (1usize << ws.q2_circuit.num_qubits).min(64);
                if self.q2_shot_counts.len() != dim {
                    self.q2_shot_counts = vec![0; dim];
                }
                self.accum_events += dt * 36.0;
                while self.accum_events >= 1.0 {
                    self.accum_events -= 1.0;
                    self.total_events += 1;
                    let sv = crate::statevector::Statevector::execute_circuit(
                        &ws.q2_circuit,
                        Some(ws.q2_step_cursor),
                        self.rng.next_u64(),
                    );
                    let u = self.rng.next_f64();
                    let mut acc = 0.0;
                    let mut chosen = 0usize;
                    for (i, amp) in sv.amps.iter().take(dim).enumerate() {
                        acc += amp.norm_sqr();
                        if u <= acc {
                            chosen = i;
                            break;
                        }
                    }
                    self.q2_shot_counts[chosen] += 1;
                    if !ws.q2_circuit.ops.is_empty() && self.rng.next_f64() < 0.08 {
                        self.q2_fault_sparks.push((
                            self.rng.next_usize(ws.q2_circuit.num_qubits),
                            self.rng.next_usize(ws.q2_circuit.ops.len()),
                            0.35,
                        ));
                    }
                }
            }
            ModuleId::Q3Teleportation => {
                self.q3_pair_phase += dt * 1.45;
                if self.q3_pair_phase >= 1.0 {
                    self.q3_pair_phase -= 1.0;
                    self.total_events += 1;
                    self.q3_last_bell = (u8::from(self.rng.next_bool()), u8::from(self.rng.next_bool()));
                    let src = [
                        ws.q3_theta.sin() * ws.q3_phi.cos(),
                        ws.q3_theta.sin() * ws.q3_phi.sin(),
                        ws.q3_theta.cos(),
                    ];
                    let bob = if self.rng.next_f64() < ws.q3_depol_p.clamp(0.0, 1.0) {
                        match self.rng.next_usize(3) {
                            0 => [src[0], -src[1], -src[2]],
                            1 => [-src[0], src[1], -src[2]],
                            _ => [-src[0], -src[1], src[2]],
                        }
                    } else {
                        src
                    };
                    if self.q3_bob_cloud.len() >= 80 {
                        self.q3_bob_cloud.pop_front();
                    }
                    self.q3_bob_cloud.push_back(bob);
                }
            }
            ModuleId::Q4OpenSystemsQec => {
                let p_rate = ws.q4_step_noise * 2.5;
                for w in &mut self.q4_walkers {
                    match ws.q4_noise_kind {
                        NoiseChannelKind::AmplitudeDampingT1 => {
                            if self.rng.next_f64() < p_rate * dt {
                                *w = [0.0, 0.0, 1.0];
                            } else {
                                w[0] *= 1.0 - 0.5 * p_rate * dt;
                                w[1] *= 1.0 - 0.5 * p_rate * dt;
                                w[2] = (w[2] + p_rate * dt).min(1.0);
                            }
                        }
                        NoiseChannelKind::PhaseDampingT2 | NoiseChannelKind::PhaseFlip => {
                            if self.rng.next_f64() < p_rate * dt {
                                w[0] = -w[0];
                                w[1] = -w[1];
                            }
                        }
                        _ => {
                            if self.rng.next_f64() < p_rate * dt {
                                let th = self.rng.next_f64() * PI;
                                let ph = self.rng.next_f64() * 2.0 * PI;
                                *w = [th.sin() * ph.cos(), th.sin() * ph.sin(), th.cos()];
                            }
                        }
                    }
                }
                self.accum_events += dt * 10.0;
                while self.accum_events >= 1.0 {
                    self.accum_events -= 1.0;
                    self.total_events += 1;
                    let p = ws.q4_physical_p.clamp(0.0, 0.5);
                    let f0 = self.rng.next_f64() < p;
                    let f1 = self.rng.next_f64() < p;
                    let f2 = self.rng.next_f64() < p;
                    self.q4_last_syndrome = (u8::from(f0 ^ f1), u8::from(f1 ^ f2));
                    if f0 {
                        self.q4_unenc_alive *= 0.96;
                    }
                    if usize::from(f0) + usize::from(f1) + usize::from(f2) >= 2 {
                        self.q4_coded_alive *= 0.96;
                    }
                    if self.q4_unenc_alive < 0.05 && self.q4_coded_alive < 0.15 {
                        self.q4_unenc_alive = 1.0;
                        self.q4_coded_alive = 1.0;
                    }
                    if self.q4_survival_history.len() >= 140 {
                        self.q4_survival_history.pop_front();
                    }
                    self.q4_survival_history.push_back((
                        self.sim_time,
                        self.q4_unenc_alive,
                        self.q4_coded_alive,
                    ));
                }
            }
            ModuleId::D1QuditWeylLab => {
                let d = ws.d1_dimension_d.clamp(2, 8);
                if self.d1_coincidences.len() != d * d {
                    self.d1_coincidences = vec![0; d * d];
                }
                for p in &mut self.d1_pulses {
                    p.x_frac += (dt * 0.85) as f32;
                }
                self.d1_pulses.retain(|p| p.x_frac < 1.0);
                self.accum_events += dt * 24.0;
                while self.accum_events >= 1.0 {
                    self.accum_events -= 1.0;
                    self.total_events += 1;
                    let j = self.rng.next_usize(d);
                    let k = (j + ws.d1_shift_a) % d;
                    self.d1_coincidences[j * d + k] += 1;
                    self.d1_pulses.push(FlyingPhoton {
                        x_frac: 0.0, lane: j, bit: j as u8, basis: k as u8,
                        intercepted: false, errored: false, erased: false,
                    });
                }
            }
            ModuleId::K1Bb84Qkd => {
                for ph in &mut self.qkd_photons {
                    ph.x_frac += (dt * 0.72) as f32;
                }
                self.qkd_photons.retain(|p| p.x_frac < 1.0);
                self.accum_events += dt * 28.0;
                while self.accum_events >= 1.0 {
                    self.accum_events -= 1.0;
                    self.total_events += 1;
                    let a_bit = u8::from(self.rng.next_bool());
                    let a_bas = u8::from(self.rng.next_bool());
                    let eve_int = self.rng.next_f64() < ws.k1_eve_rate;
                    let e_bas = u8::from(self.rng.next_bool());
                    let (tx_bit, tx_bas) = if eve_int {
                        (if e_bas == a_bas { a_bit } else { u8::from(self.rng.next_bool()) }, e_bas)
                    } else {
                        (a_bit, a_bas)
                    };
                    let b_bas = u8::from(self.rng.next_bool());
                    let mut b_bit = if b_bas == tx_bas { tx_bit } else { u8::from(self.rng.next_bool()) };
                    if self.rng.next_f64() < ws.k1_channel_noise {
                        b_bit ^= 1;
                    }
                    let keep = a_bas == b_bas;
                    let err = keep && (a_bit != b_bit);
                    let basis_char = |b: u8| if b == 0 { '+' } else { 'x' };
                    if keep {
                        self.k1_sifted_total += 1;
                        if err { self.k1_sifted_errors += 1; }
                        if self.k1_bit_tape.len() >= 28 { self.k1_bit_tape.pop_front(); }
                        self.k1_bit_tape.push_back(Bb84TapeEntry {
                            pulse_index: self.total_events,
                            alice_bit: a_bit,
                            alice_basis: basis_char(a_bas),
                            eve_intercepted: eve_int,
                            eve_basis: if eve_int { basis_char(e_bas) } else { '-' },
                            bob_basis: basis_char(b_bas),
                            bob_bit: b_bit,
                            sifted_keep: true,
                            bit_error: err,
                        });
                    }
                    self.qkd_photons.push(FlyingPhoton {
                        x_frac: 0.0, lane: a_bas as usize, bit: a_bit, basis: b_bas,
                        intercepted: eve_int, errored: err, erased: !keep,
                    });
                }
                if self.k1_sifted_total > 0 {
                    let qber = (self.k1_sifted_errors as f64) * 100.0 / (self.k1_sifted_total as f64);
                    if self.k1_qber_trace.len() >= 150 { self.k1_qber_trace.pop_front(); }
                    self.k1_qber_trace.push_back((self.sim_time, qber));
                }
            }
            ModuleId::K2E91BellChsh => {
                self.accum_events += dt * 45.0;
                let a_ang = [0.0, PI / 4.0, PI / 2.0];
                let b_ang = [PI / 4.0, PI / 2.0, 3.0 * PI / 4.0];
                while self.accum_events >= 1.0 {
                    self.accum_events -= 1.0;
                    self.total_events += 1;
                    let ia = self.rng.next_usize(3);
                    let ib = self.rng.next_usize(3);
                    let (sa, sb) = if self.rng.next_f64() < ws.k2_eve_decoherence {
                        let phi = self.rng.next_f64() * PI;
                        let eo = if self.rng.next_bool() { 1.0 } else { -1.0 };
                        let pa = 0.5 * (1.0 + ws.k2_visibility * eo * (a_ang[ia] - phi).cos());
                        let pb = 0.5 * (1.0 - ws.k2_visibility * eo * (b_ang[ib] - phi).cos());
                        (
                            if self.rng.next_f64() < pa { 1.0 } else { -1.0 },
                            if self.rng.next_f64() < pb { 1.0 } else { -1.0 },
                        )
                    } else {
                        let s0 = if self.rng.next_bool() { 1.0 } else { -1.0 };
                        let c = -ws.k2_visibility * (a_ang[ia] - b_ang[ib]).cos();
                        let ps = (0.5 * (1.0 + c)).clamp(0.0, 1.0);
                        (s0, if self.rng.next_f64() < ps { s0 } else { -s0 })
                    };
                    self.k2_corr_sum[ia][ib] += sa * sb;
                    self.k2_corr_cnt[ia][ib] += 1;
                }
                let e = |r: usize, c: usize| -> f64 {
                    if self.k2_corr_cnt[r][c] > 0 {
                        self.k2_corr_sum[r][c] / (self.k2_corr_cnt[r][c] as f64)
                    } else {
                        0.0
                    }
                };
                let s_live = (e(0, 0) - e(0, 2) + e(2, 0) + e(2, 2)).abs();
                if self.k2_chsh_trace.len() >= 150 { self.k2_chsh_trace.pop_front(); }
                self.k2_chsh_trace.push_back((self.sim_time, s_live));
            }
            ModuleId::K3B92NonOrthogonal => {
                let sin2 = (ws.k3_angle_deg * PI / 180.0).sin().powi(2);
                self.accum_events += dt * 35.0;
                while self.accum_events >= 1.0 {
                    self.accum_events -= 1.0;
                    self.total_events += 1;
                    let a_bit = u8::from(self.rng.next_bool());
                    let tx_bit = if self.rng.next_f64() < ws.k3_eve_rate {
                        let et = u8::from(self.rng.next_bool());
                        if et != a_bit && self.rng.next_f64() < sin2 { a_bit } else { u8::from(self.rng.next_bool()) }
                    } else {
                        a_bit
                    };
                    let b_test = u8::from(self.rng.next_bool());
                    if b_test == tx_bit && self.rng.next_f64() < sin2 {
                        self.k3_conclusive += 1;
                        if b_test != a_bit { self.k3_errors += 1; }
                    } else {
                        self.k3_erased += 1;
                    }
                }
                let tot = self.k3_conclusive + self.k3_erased;
                if tot > 0 {
                    let y_pct = (self.k3_conclusive as f64) * 100.0 / (tot as f64);
                    if self.k3_yield_trace.len() >= 150 { self.k3_yield_trace.pop_front(); }
                    self.k3_yield_trace.push_back((self.sim_time, y_pct));
                }
            }
            ModuleId::C1ShorRsa => {
                let rep = run_shor_cryptanalysis(ws.c1_modulus_n, ws.c1_base_a, ws.c1_counting_qubits);
                let buckets = 64usize.min(rep.qft_spectrum.len());
                if self.c1_bin_counts.len() != buckets {
                    self.c1_bin_counts = vec![0; buckets];
                }
                let csz = (rep.qft_spectrum.len() / buckets).max(1);
                self.accum_events += dt * 25.0;
                while self.accum_events >= 1.0 {
                    self.accum_events -= 1.0;
                    self.total_events += 1;
                    let u = self.rng.next_f64();
                    let mut cdf = 0.0;
                    let mut y_pick = 0usize;
                    for (i, &p) in rep.qft_spectrum.iter().enumerate() {
                        cdf += p;
                        if u <= cdf { y_pick = i; break; }
                    }
                    self.c1_bin_counts[(y_pick / csz).min(buckets - 1)] += 1;
                    let g = gcd_u64(ws.c1_base_a, ws.c1_modulus_n);
                    if g > 1 && g < ws.c1_modulus_n {
                        self.c1_locked_r = None;
                        self.c1_locked_factors = rep.recovered_factors;
                    } else if mod_pow_u64(ws.c1_base_a, rep.quantum_period_r, ws.c1_modulus_n) == 1
                    {
                        self.c1_locked_r = Some(rep.quantum_period_r);
                        self.c1_locked_factors = rep.recovered_factors;
                    }
                }
            }
            ModuleId::C2GroverSymmetric => {
                self.c2_step_timer += dt * 2.5;
                if self.c2_step_timer >= 1.0 {
                    self.c2_step_timer -= 1.0;
                    self.total_events += 1;
                    self.c2_live_iter = (self.c2_live_iter + 1) % 29;
                }
            }
            ModuleId::C3MlKemLattice => {
                self.accum_events += dt * 12.0;
                while self.accum_events >= 1.0 {
                    self.accum_events -= 1.0;
                    self.total_events += 1;
                    let rep = run_ml_kem_lab(ws.c3_profile, ws.c3_eta, self.rng.next_u64());
                    self.c3_handshakes += 1;
                    if !rep.decapsulation_exact { self.c3_failed_handshakes += 1; }
                    if self.c3_max_noise_history.len() >= 140 { self.c3_max_noise_history.pop_front(); }
                    self.c3_max_noise_history.push_back((self.sim_time, rep.max_abs_decryption_noise as f64));
                    if self.c3_lwe_cloud.len() >= 90 { self.c3_lwe_cloud.pop_front(); }
                    self.c3_lwe_cloud.push_back(rep.lattice_2d_noisy_lwe);
                }
            }
            ModuleId::C4PqcSignatures => {
                let g1 = ws.c4_gamma1.clamp(32, 512);
                let eta = ws.c4_eta.clamp(1, 8);
                let tau = ws.c4_tau.clamp(1, 16);
                let beta = (tau as i32) * eta;
                let safe = (g1 - beta).max(4);
                let bmax = g1 + beta;
                self.accum_events += dt * 60.0;
                while self.accum_events >= 1.0 {
                    self.accum_events -= 1.0;
                    self.total_events += 1;
                    self.c4_trials += 1;
                    let mut ok = true;
                    let mut z0 = 0i32;
                    for d_i in 0..16 {
                        let s_i = (self.rng.next_usize((2 * eta + 1) as usize) as i32) - eta;
                        let y_i = (self.rng.next_usize((2 * g1) as usize) as i32) - g1 + 1;
                        let cs = (s_i * (self.rng.next_usize(tau + 1) as i32)).clamp(-beta, beta);
                        let z = y_i + cs;
                        if d_i == 0 { z0 = z; }
                        if z.abs() >= safe { ok = false; }
                    }
                    let bin = (((z0 + bmax) as f64 / (2 * bmax).max(1) as f64) * 21.0)
                        .floor()
                        .clamp(0.0, 20.0) as usize;
                    self.c4_raw_bins[bin] += 1;
                    if ok {
                        self.c4_accepted += 1;
                        self.c4_acc_bins[bin] += 1;
                    }
                }
            }
            ModuleId::A1SurfaceCode => {
                let d = match ws.a1_code_distance {
                    0..=3 => 3,
                    4..=5 => 5,
                    _ => 7,
                };
                if self.a1_data_errors.len() != d * d {
                    self.a1_data_errors = vec![0; d * d];
                }
                let p = ws.a1_phys_error.clamp(0.0005, 0.04);
                let p_th = 0.010; // ~1.0% Fowler et al. (2012) circuit-level surface-code threshold
                self.accum_events += dt * 14.0;
                while self.accum_events >= 1.0 {
                    self.accum_events -= 1.0;
                    self.total_events += 1;
                    for cell in &mut self.a1_data_errors {
                        let r = self.rng.next_f64();
                        *cell = if r < p * 4.0 {
                            if self.rng.next_bool() { 1 } else { 2 }
                        } else {
                            0
                        };
                    }
                    // Fowler scaling P_L(d) = 0.08 * (p / p_th)^((d + 1)/2)
                    let pl3 = (0.08 * (p / p_th).powi(2)).min(0.45);
                    let pl5 = (0.08 * (p / p_th).powi(3)).min(0.45);
                    let pl7 = (0.08 * (p / p_th).powi(4)).min(0.45);
                    self.a1_surv_d3 *= 1.0 - pl3;
                    self.a1_surv_d5 *= 1.0 - pl5;
                    self.a1_surv_d7 *= 1.0 - pl7;
                    if self.a1_surv_d3 < 0.08 && self.a1_surv_d7 < 0.08 {
                        self.a1_surv_d3 = 1.0;
                        self.a1_surv_d5 = 1.0;
                        self.a1_surv_d7 = 1.0;
                    }
                    if self.a1_trace.len() >= 140 {
                        self.a1_trace.pop_front();
                    }
                    self.a1_trace.push_back((
                        self.sim_time,
                        self.a1_surv_d3,
                        self.a1_surv_d5,
                        self.a1_surv_d7,
                    ));
                }
            }
            ModuleId::A2VqeHydrogen => {
                self.accum_events += dt * 16.0;
                while self.accum_events >= 1.0 {
                    self.accum_events -= 1.0;
                    self.total_events += 1;
                    let delta = 0.08;
                    let noise_p = (self.rng.next_f64() - 0.5) * 2.0 * ws.a2_shot_noise;
                    let noise_m = (self.rng.next_f64() - 0.5) * 2.0 * ws.a2_shot_noise;
                    let (fci, e_plus) = h2_fci_and_ansatz_energy(ws.a2_bond_r_angstrom, self.a2_theta + delta);
                    let (_, e_minus) = h2_fci_and_ansatz_energy(ws.a2_bond_r_angstrom, self.a2_theta - delta);
                    let grad = ((e_plus + noise_p) - (e_minus + noise_m)) / (2.0 * delta);
                    self.a2_theta = (self.a2_theta - 0.18 * grad).clamp(-PI, PI);
                    let (_, e_cur) = h2_fci_and_ansatz_energy(ws.a2_bond_r_angstrom, self.a2_theta);
                    if self.a2_energy_trace.len() >= 140 {
                        self.a2_energy_trace.pop_front();
                    }
                    self.a2_energy_trace.push_back((self.sim_time, e_cur, fci));
                }
            }
            ModuleId::A3QaoaMaxCut => {
                // 8-node cubic/ring-chord Rydberg graph with 12 edges (optimal Max-Cut C_max = 10)
                const EDGES: [(usize, usize); 12] = [
                    (0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 6),
                    (6, 7), (7, 0), (0, 4), (1, 5), (2, 6), (3, 7),
                ];
                let p_depth = ws.a3_depth_p.clamp(1, 6) as f64;
                let resonance = (ws.a3_drive_gamma * PI * 0.5).sin().abs();
                let beta_temp = 0.35 + 0.55 * p_depth * resonance;

                self.accum_events += dt * 30.0;
                while self.accum_events >= 1.0 {
                    self.accum_events -= 1.0;
                    self.total_events += 1;
                    // Sample bitstring weighted by QAOA interference concentration exp(beta_temp * (cut - 6))
                    let mut best_m = 0u8;
                    let mut chosen_cut = 0usize;
                    for _ in 0..8 {
                        let cand = (self.rng.next_u32() & 0xFF) as u8;
                        let mut cut = 0usize;
                        for &(u, v) in &EDGES {
                            if ((cand >> u) & 1) != ((cand >> v) & 1) {
                                cut += 1;
                            }
                        }
                        let weight = (beta_temp * ((cut as f64) - 10.0)).exp();
                        if self.rng.next_f64() <= weight {
                            best_m = cand;
                            chosen_cut = cut;
                            break;
                        }
                        best_m = cand;
                        chosen_cut = cut;
                    }
                    self.a3_last_mask = best_m;
                    let idx = chosen_cut.min(9);
                    self.a3_cut_bins[idx] += 1;
                }
                let tot_shots: u64 = self.a3_cut_bins.iter().sum();
                if tot_shots > 0 {
                    let mean_cut: f64 = self
                        .a3_cut_bins
                        .iter()
                        .enumerate()
                        .map(|(c, &n)| (c as f64) * (n as f64))
                        .sum::<f64>()
                        / (tot_shots as f64);
                    if self.a3_ratio_trace.len() >= 140 {
                        self.a3_ratio_trace.pop_front();
                    }
                    self.a3_ratio_trace.push_back((self.sim_time, mean_cut / 10.0));
                }
            }
            ModuleId::A4MagicStateFactory => {
                let p_in = ws.a4_raw_t_error.clamp(0.002, 0.12);
                self.accum_events += dt * 32.0;
                while self.accum_events >= 1.0 {
                    self.accum_events -= 1.0;
                    self.total_events += 1;
                    self.a4_batches += 1;
                    // 15 noisy |T> inputs: 0 errors -> clean output; 1-2 errors -> detected by [[15,1,3]] syndrome & aborted;
                    // weight-3 undetected stabilizer logical error occurs on 35 triples out of C(15, 3) = 455 (~35 * p^3)!
                    let mut errs = 0usize;
                    for _ in 0..15 {
                        if self.rng.next_f64() < p_in {
                            errs += 1;
                        }
                    }
                    if errs == 0 {
                        self.a4_distilled_ok += 1;
                    } else if errs >= 3 && self.rng.next_f64() < (35.0 / 455.0) {
                        self.a4_distilled_err += 1;
                    } else {
                        self.a4_aborted_blocks += 1;
                    }
                }
                let out_tot = self.a4_distilled_ok + self.a4_distilled_err;
                if out_tot > 0 {
                    let emp_p_out = (self.a4_distilled_err as f64) / (out_tot as f64);
                    if self.a4_err_trace.len() >= 140 {
                        self.a4_err_trace.pop_front();
                    }
                    self.a4_err_trace.push_back((self.sim_time, emp_p_out * 100.0));
                }
            }
            ModuleId::Q5EntanglementSwapping => {
                let hops = ws.q5_repeater_hops.clamp(1, 4);
                let p_link = ws.q5_link_depol.clamp(0.0, 0.25);
                let w_link = (1.0 - 4.0 * p_link / 3.0).max(0.0);
                let w_end = w_link.powi((hops as i32) + 1);
                let f_bell = 0.25 + 0.75 * w_end;
                self.accum_events += dt * 26.0;
                while self.accum_events >= 1.0 {
                    self.accum_events -= 1.0;
                    self.total_events += 1;
                    self.q5_trials += 1;
                    self.q5_swap_flags.resize(hops, false);
                    let mut all_ok = true;
                    for slot in &mut self.q5_swap_flags {
                        // Linear-optical passive beam-splitter BSM resolves 2 of 4 Bell states (50% herald per hop)
                        let ok = if ws.q5_linear_optics_bsm {
                            self.rng.next_bool()
                        } else {
                            true
                        };
                        *slot = ok;
                        if !ok {
                            all_ok = false;
                        }
                    }
                    if all_ok {
                        self.q5_heralded += 1;
                        if self.rng.next_f64() < f_bell {
                            self.q5_bell_matches += 1;
                        }
                    }
                }
                if self.q5_heralded > 0 {
                    let emp_fid = (self.q5_bell_matches as f64) / (self.q5_heralded as f64);
                    if self.q5_fid_trace.len() >= 140 {
                        self.q5_fid_trace.pop_front();
                    }
                    self.q5_fid_trace.push_back((self.sim_time, emp_fid));
                }
            }
            ModuleId::D2QuditTeleportation => {
                let d = ws.d2_dimension_d.clamp(2, 8);
                let tilt = ws.d2_mode_tilt.clamp(0.0, 1.2);
                let p_dep = ws.d2_depol_p.clamp(0.0, 0.90);
                if self.d2_bob_counts.len() != d {
                    self.d2_bob_counts = vec![0; d];
                }
                let mut weights = vec![0.0; d];
                let mut z_sum = 0.0;
                for (k, w) in weights.iter_mut().enumerate() {
                    *w = (-2.0 * tilt * (k as f64)).exp();
                    z_sum += *w;
                }
                for w in &mut weights {
                    *w /= z_sum.max(1e-12);
                }

                self.accum_events += dt * 30.0;
                while self.accum_events >= 1.0 {
                    self.accum_events -= 1.0;
                    self.total_events += 1;
                    self.d2_shots += 1;
                    let m = self.rng.next_usize(d);
                    let n = self.rng.next_usize(d);
                    self.d2_last_mn = (m, n);

                    if self.rng.next_f64() >= p_dep {
                        // Clean Weyl-Heisenberg Z_d^m X_d^n recovery reproduces Alice's d-mode profile
                        let u = self.rng.next_f64();
                        let mut cdf = 0.0;
                        let mut picked = d - 1;
                        for (k, &wk) in weights.iter().enumerate() {
                            cdf += wk;
                            if u <= cdf {
                                picked = k;
                                break;
                            }
                        }
                        self.d2_bob_counts[picked] += 1;
                        self.d2_fid_sum += 1.0;
                    } else {
                        // Isotropic d-dimensional white noise mixes uniformly across |0>..|d-1>
                        let k = self.rng.next_usize(d);
                        self.d2_bob_counts[k] += 1;
                        self.d2_fid_sum += 1.0 / (d as f64);
                    }
                }
                if self.d2_shots > 0 {
                    let avg_f = self.d2_fid_sum / (self.d2_shots as f64);
                    if self.d2_fid_trace.len() >= 140 {
                        self.d2_fid_trace.pop_front();
                    }
                    self.d2_fid_trace.push_back((self.sim_time, avg_f));
                }
            }
            ModuleId::A5GateTeleportation => {
                let depth = ws.a5_chain_depth.clamp(1, 16);
                let p_raw = ws.a5_raw_ancilla_p.clamp(0.002, 0.15);
                let p_eff = if ws.a5_use_distilled {
                    (35.0 * p_raw.powi(3) + 0.0008).min(p_raw)
                } else {
                    p_raw
                };
                let target_phi = (depth as f64) * (PI * 0.25);
                let raw_baseline_fid = 0.5 * (1.0 + (1.0 - 2.0 * p_raw).powi(depth as i32));

                self.accum_events += dt * 24.0;
                while self.accum_events >= 1.0 {
                    self.accum_events -= 1.0;
                    self.total_events += 1;
                    self.a5_shots += 1;
                    let mut phi = 0.0_f64;
                    for _ in 0..depth {
                        self.a5_total_gadgets += 1;
                        let m_anc = self.rng.next_bool();
                        if m_anc {
                            self.a5_s_fixups += 1;
                            // m = 1 projects to phi - pi/4, then Clifford S^1 adds +pi/2 -> net +pi/4
                            phi += -PI * 0.25 + PI * 0.50;
                        } else {
                            phi += PI * 0.25;
                        }
                        if self.rng.next_f64() < p_eff {
                            // Faulty |T> ancilla injects stochastic Z phase flip
                            phi += PI * (0.85 + 0.30 * self.rng.next_f64());
                        }
                    }
                    let bx = phi.cos();
                    let by = phi.sin();
                    let bz = (self.rng.next_f64() - 0.5) * 0.06;
                    if self.a5_cloud.len() >= 96 {
                        self.a5_cloud.pop_front();
                    }
                    self.a5_cloud.push_back([bx, by, bz]);
                    let shot_fid = 0.5 * (1.0 + (phi - target_phi).cos());
                    self.a5_fid_sum += shot_fid;
                }
                if self.a5_shots > 0 {
                    let emp_fid = self.a5_fid_sum / (self.a5_shots as f64);
                    if self.a5_fid_trace.len() >= 140 {
                        self.a5_fid_trace.pop_front();
                    }
                    self.a5_fid_trace.push_back((self.sim_time, emp_fid, raw_baseline_fid));
                }
            }
        }
    }
}
