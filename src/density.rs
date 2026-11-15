//! Open quantum systems, Kraus operator channels (T1 amplitude damping, T2 phase damping,
//! depolarizing, bit-flip, phase-flip), Ramsey decoherence curves, and 3-qubit Quantum Error Correction.

use crate::complex::{rho_to_bloch, CMatrix, Complex64};
use crate::statevector::{GateOp, Statevector};
use crate::rng_sha256::DeterministicRng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoiseChannelKind {
    AmplitudeDampingT1,
    PhaseDampingT2,
    Depolarizing,
    BitFlip,
    PhaseFlip,
}

impl NoiseChannelKind {
    pub const ALL: [Self; 5] = [
        Self::AmplitudeDampingT1,
        Self::PhaseDampingT2,
        Self::Depolarizing,
        Self::BitFlip,
        Self::PhaseFlip,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::AmplitudeDampingT1 => "Amplitude Damping (T1 spontaneous emission)",
            Self::PhaseDampingT2 => "Phase Damping (T2 pure dephasing)",
            Self::Depolarizing => "Symmetric Depolarizing Channel",
            Self::BitFlip => "Stochastic Bit-Flip (Pauli X)",
            Self::PhaseFlip => "Stochastic Phase-Flip (Pauli Z)",
        }
    }
}

/// Returns single-qubit 2x2 Kraus operators `{E_k}` satisfying `sum_k E_k^dagger E_k = I_2`.
pub fn single_qubit_kraus(kind: NoiseChannelKind, param: f64) -> Vec<CMatrix> {
    let p = param.clamp(0.0, 1.0);
    match kind {
        NoiseChannelKind::AmplitudeDampingT1 => {
            // E0 = [[1, 0], [0, sqrt(1-gamma)]], E1 = [[0, sqrt(gamma)], [0, 0]]
            let s1 = (1.0 - p).sqrt();
            let sg = p.sqrt();
            let e0 = CMatrix::from_rows(
                2,
                vec![
                    Complex64::ONE,
                    Complex64::ZERO,
                    Complex64::ZERO,
                    Complex64::new(s1, 0.0),
                ],
            );
            let e1 = CMatrix::from_rows(
                2,
                vec![
                    Complex64::ZERO,
                    Complex64::new(sg, 0.0),
                    Complex64::ZERO,
                    Complex64::ZERO,
                ],
            );
            vec![e0, e1]
        }
        NoiseChannelKind::PhaseDampingT2 => {
            // E0 = [[1, 0], [0, sqrt(1-lambda)]], E1 = [[0, 0], [0, sqrt(lambda)]]
            let s1 = (1.0 - p).sqrt();
            let sl = p.sqrt();
            let e0 = CMatrix::from_rows(
                2,
                vec![
                    Complex64::ONE,
                    Complex64::ZERO,
                    Complex64::ZERO,
                    Complex64::new(s1, 0.0),
                ],
            );
            let e1 = CMatrix::from_rows(
                2,
                vec![
                    Complex64::ZERO,
                    Complex64::ZERO,
                    Complex64::ZERO,
                    Complex64::new(sl, 0.0),
                ],
            );
            vec![e0, e1]
        }
        NoiseChannelKind::Depolarizing => {
            // rho -> (1 - 3p/4) I rho I + (p/4)(X rho X + Y rho Y + Z rho Z)
            // Parameterized so p = 1 yields maximally mixed state I/2!
            let w0 = (1.0 - 0.75 * p).max(0.0).sqrt();
            let wp = (0.25 * p).sqrt();
            let e_i = CMatrix::identity(2).scale(Complex64::new(w0, 0.0));
            let e_x = CMatrix::from_rows(
                2,
                vec![
                    Complex64::ZERO,
                    Complex64::new(wp, 0.0),
                    Complex64::new(wp, 0.0),
                    Complex64::ZERO,
                ],
            );
            let e_y = CMatrix::from_rows(
                2,
                vec![
                    Complex64::ZERO,
                    Complex64::new(0.0, -wp),
                    Complex64::new(0.0, wp),
                    Complex64::ZERO,
                ],
            );
            let e_z = CMatrix::from_rows(
                2,
                vec![
                    Complex64::new(wp, 0.0),
                    Complex64::ZERO,
                    Complex64::ZERO,
                    Complex64::new(-wp, 0.0),
                ],
            );
            vec![e_i, e_x, e_y, e_z]
        }
        NoiseChannelKind::BitFlip => {
            let w0 = (1.0 - p).sqrt();
            let w1 = p.sqrt();
            let e0 = CMatrix::identity(2).scale(Complex64::new(w0, 0.0));
            let e1 = CMatrix::from_rows(
                2,
                vec![
                    Complex64::ZERO,
                    Complex64::new(w1, 0.0),
                    Complex64::new(w1, 0.0),
                    Complex64::ZERO,
                ],
            );
            vec![e0, e1]
        }
        NoiseChannelKind::PhaseFlip => {
            let w0 = (1.0 - p).sqrt();
            let w1 = p.sqrt();
            let e0 = CMatrix::identity(2).scale(Complex64::new(w0, 0.0));
            let e1 = CMatrix::from_rows(
                2,
                vec![
                    Complex64::new(w1, 0.0),
                    Complex64::ZERO,
                    Complex64::ZERO,
                    Complex64::new(-w1, 0.0),
                ],
            );
            vec![e0, e1]
        }
    }
}

/// Exact n-qubit density matrix `rho` (`1..=5` qubits, `dim = 2^n`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DensityMatrix {
    pub num_qubits: usize,
    pub mat: CMatrix,
}

impl DensityMatrix {
    pub fn from_statevector(sv: &Statevector) -> Self {
        assert!(sv.num_qubits <= 6, "Density matrix capped at 6 qubits");
        let d = sv.dim();
        let mut mat = CMatrix::zeros(d);
        for r in 0..d {
            for c in 0..d {
                mat.set(r, c, sv.amps[r] * sv.amps[c].conj());
            }
        }
        Self {
            num_qubits: sv.num_qubits,
            mat,
        }
    }

    pub fn maximally_mixed(num_qubits: usize) -> Self {
        let n = num_qubits.clamp(1, 6);
        let d = 1usize << n;
        let mat = CMatrix::identity(d).scale(Complex64::new(1.0 / (d as f64), 0.0));
        Self { num_qubits: n, mat }
    }

    /// Embeds a single-qubit 2x2 operator `op` on `target` (where qubit 0 is LSB) via tensor product.
    pub fn embed_single_qubit_op(num_qubits: usize, target: usize, op: &CMatrix) -> CMatrix {
        assert_eq!(op.dim, 2);
        assert!(target < num_qubits);
        // Since index bit `q` corresponds to `1 << q`, MSB is `num_qubits - 1` and LSB is `0`:
        // Full matrix is op_{n-1} (x) op_{n-2} (x) ... (x) op_0.
        let id2 = CMatrix::identity(2);
        let mut acc = if (num_qubits - 1) == target {
            op.clone()
        } else {
            id2.clone()
        };
        for q in (0..num_qubits - 1).rev() {
            let next = if q == target { op } else { &id2 };
            acc = acc.kron(next);
        }
        acc
    }

    /// Applies a Kraus channel `{E_k}` to `target` qubit: `rho' = sum_k K_k rho K_k^dagger`.
    pub fn apply_single_qubit_channel(&mut self, target: usize, kind: NoiseChannelKind, param: f64) {
        let kraus_2x2 = single_qubit_kraus(kind, param);
        let mut next = CMatrix::zeros(self.mat.dim);
        for e2 in &kraus_2x2 {
            let k_full = Self::embed_single_qubit_op(self.num_qubits, target, e2);
            let k_dag = k_full.dagger();
            let term = k_full.matmul(&self.mat).matmul(&k_dag);
            next = next.add_mat(&term);
        }
        self.mat = next;
    }

    pub fn purity(&self) -> f64 {
        self.mat.purity()
    }

    pub fn entropy_bits(&self) -> f64 {
        self.mat.von_neumann_entropy_bits()
    }

    /// Fidelity `F(|psi>, rho) = <psi| rho |psi>` against a reference pure statevector.
    pub fn fidelity_with_pure(&self, sv: &Statevector) -> f64 {
        assert_eq!(self.mat.dim, sv.dim());
        let r_psi = self.mat.apply_vec(&sv.amps);
        let mut acc = Complex64::ZERO;
        for (bra, ket) in sv.amps.iter().zip(r_psi.iter()) {
            acc += bra.conj() * (*ket);
        }
        acc.re.clamp(0.0, 1.0)
    }
}

/// Trajectory point for single-qubit open-system Lindblad/Kraus evolution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecoherencePoint {
    pub step: usize,
    pub bloch: [f64; 3],
    pub purity: f64,
    pub entropy_bits: f64,
    pub fidelity: f64,
}

pub fn simulate_decoherence_trajectory(
    theta0: f64,
    phi0: f64,
    kind: NoiseChannelKind,
    step_noise: f64,
    steps: usize,
) -> (CMatrix, Vec<DecoherencePoint>) {
    let mut sv0 = Statevector::zero_state(1);
    let mut rng = DeterministicRng::from_seed(7);
    sv0.apply_op(&GateOp::Ry { target: 0, theta: theta0 }, &mut rng);
    sv0.apply_op(&GateOp::Rz { target: 0, theta: phi0 }, &mut rng);

    let mut dm = DensityMatrix::from_statevector(&sv0);
    let mut history = Vec::with_capacity(steps + 1);
    for step in 0..=steps {
        let bloch = rho_to_bloch(&dm.mat);
        history.push(DecoherencePoint {
            step,
            bloch,
            purity: dm.purity(),
            entropy_bits: dm.entropy_bits(),
            fidelity: dm.fidelity_with_pure(&sv0),
        });
        if step < steps {
            dm.apply_single_qubit_channel(0, kind, step_noise);
        }
    }
    (dm.mat, history)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum QecCodeKind {
    ThreeQubitBitFlip,
    ThreeQubitPhaseFlip,
}

impl QecCodeKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::ThreeQubitBitFlip => "3-Qubit Bit-Flip Code (|0_L> = |000>, |1_L> = |111>)",
            Self::ThreeQubitPhaseFlip => "3-Qubit Phase-Flip Code (|0_L> = |+++>, |1_L> = |--->)",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QecSimulationSummary {
    pub code: QecCodeKind,
    pub physical_error_p: f64,
    pub unencoded_fidelity: f64,
    pub logical_recovered_fidelity: f64,
    pub injected_fault_qubit: Option<usize>,
    pub syndrome_bits: (u8, u8),
    pub recovery_action: String,
    pub single_fault_recovered_exact: bool,
    /// Curve `(p, unencoded_fail_prob, coded_fail_prob)` for `p in 0..=0.5`.
    pub threshold_curve: Vec<(f64, f64, f64)>,
}

/// Evaluates both deterministic syndrome decoding for a chosen single-qubit fault (`injected_qubit`)
/// AND exact logical failure rate `3 p^2 (1 - p) + p^3` vs unencoded `p` across physical error rates!
pub fn simulate_three_qubit_qec(
    code: QecCodeKind,
    alpha_theta: f64,
    alpha_phi: f64,
    physical_error_p: f64,
    injected_qubit: Option<usize>,
) -> QecSimulationSummary {
    let mut rng = DeterministicRng::from_seed(42);

    // Prepare 5-qubit statevector: q0, q1, q2 = data block; q3, q4 = Z0Z1 & Z1Z2 syndrome ancillas
    let mut sv = Statevector::zero_state(5);
    sv.apply_op(&GateOp::Ry { target: 0, theta: alpha_theta }, &mut rng);
    sv.apply_op(&GateOp::Rz { target: 0, theta: alpha_phi }, &mut rng);

    let mut ref_1q = Statevector::zero_state(1);
    ref_1q.apply_op(&GateOp::Ry { target: 0, theta: alpha_theta }, &mut rng);
    ref_1q.apply_op(&GateOp::Rz { target: 0, theta: alpha_phi }, &mut rng);
    let ref_bloch = ref_1q.bloch_vector(0);

    // Encode logical state onto (q0, q1, q2)
    sv.apply_op(&GateOp::Cnot { control: 0, target: 1 }, &mut rng);
    sv.apply_op(&GateOp::Cnot { control: 0, target: 2 }, &mut rng);
    if code == QecCodeKind::ThreeQubitPhaseFlip {
        for q in 0..3 {
            sv.apply_op(&GateOp::H(q), &mut rng);
        }
    }

    // Inject deterministic fault on chosen physical data qubit (0, 1, or 2)
    if let Some(fq) = injected_qubit {
        let q = fq.min(2);
        match code {
            QecCodeKind::ThreeQubitBitFlip => sv.apply_op(&GateOp::X(q), &mut rng),
            QecCodeKind::ThreeQubitPhaseFlip => sv.apply_op(&GateOp::Z(q), &mut rng),
        }
    }

    // Rotate phase-flip code back into Z basis before parity-check ancillas
    if code == QecCodeKind::ThreeQubitPhaseFlip {
        for q in 0..3 {
            sv.apply_op(&GateOp::H(q), &mut rng);
        }
    }

    // Parity checks: q3 measures Z0 * Z1, q4 measures Z1 * Z2
    sv.apply_op(&GateOp::Cnot { control: 0, target: 3 }, &mut rng);
    sv.apply_op(&GateOp::Cnot { control: 1, target: 3 }, &mut rng);
    sv.apply_op(&GateOp::Cnot { control: 1, target: 4 }, &mut rng);
    sv.apply_op(&GateOp::Cnot { control: 2, target: 4 }, &mut rng);

    let s01 = sv.measure_qubit(3, &mut rng);
    let s12 = sv.measure_qubit(4, &mut rng);

    let recovery_action = match (s01, s12) {
        (0, 0) => "Syndrome (0, 0): No single-qubit error detected -> Identity".to_string(),
        (1, 0) => {
            sv.apply_op(&GateOp::X(0), &mut rng);
            "Syndrome (1, 0): Fault isolated on q[0] -> Apply corrective flip on q[0]".to_string()
        }
        (1, 1) => {
            sv.apply_op(&GateOp::X(1), &mut rng);
            "Syndrome (1, 1): Fault isolated on q[1] -> Apply corrective flip on q[1]".to_string()
        }
        (0, 1) => {
            sv.apply_op(&GateOp::X(2), &mut rng);
            "Syndrome (0, 1): Fault isolated on q[2] -> Apply corrective flip on q[2]".to_string()
        }
        _ => unreachable!(),
    };

    // Uncompute encoding CNOTs to decode logical qubit back into q0
    sv.apply_op(&GateOp::Cnot { control: 0, target: 2 }, &mut rng);
    sv.apply_op(&GateOp::Cnot { control: 0, target: 1 }, &mut rng);

    let decoded_bloch = sv.bloch_vector(0);
    let dot = ref_bloch[0] * decoded_bloch[0]
        + ref_bloch[1] * decoded_bloch[1]
        + ref_bloch[2] * decoded_bloch[2];
    let single_fault_fid = 0.5 * (1.0 + dot);

    let p = physical_error_p.clamp(0.0, 0.5);
    let coded_fail_p = 3.0 * p * p * (1.0 - p) + p * p * p;
    let unencoded_fidelity = 1.0 - p;
    let logical_recovered_fidelity = 1.0 - coded_fail_p;

    let mut threshold_curve = Vec::with_capacity(51);
    for i in 0..=50 {
        let px = (i as f64) * 0.01;
        let p_log = 3.0 * px * px * (1.0 - px) + px * px * px;
        threshold_curve.push((px, px, p_log));
    }

    QecSimulationSummary {
        code,
        physical_error_p: p,
        unencoded_fidelity,
        logical_recovered_fidelity,
        injected_fault_qubit: injected_qubit,
        syndrome_bits: (s01, s12),
        recovery_action,
        single_fault_recovered_exact: single_fault_fid > 0.999_999,
        threshold_curve,
    }
}
