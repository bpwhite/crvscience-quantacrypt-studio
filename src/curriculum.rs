//! 23-module passive stochastic observatory curriculum across 6 groups (`v2.2.0`):
//! - Group 0: Basic Quantum Mechanics & Wave-Particle Intro (`I1`..`I4`)
//! - Group 1: Qubit Foundations, Teleportation & Swapping (`Q1`..`Q5`)
//! - Group 2: High-Dimensional Optical Qudits (`D1`..`D2`)
//! - Group 3: Photonic Quantum Key Distribution & Bell Streams (`K1`..`K3`)
//! - Group 4: Quantum Cryptanalysis & NIST Post-Quantum Streams (`C1`..`C4`)
//! - Group 5: Advanced Fault-Tolerant & Variational Computing (`A1`..`A5`)

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CurriculumGroup {
    BasicQuantumIntro,
    FoundationsOpenSystems,
    HighDimensionalQudits,
    QuantumKeyDistribution,
    CryptanalysisAndPqc,
    AdvancedQuantumComputing,
}

impl CurriculumGroup {
    pub const ALL: [Self; 6] = [
        Self::BasicQuantumIntro,
        Self::FoundationsOpenSystems,
        Self::HighDimensionalQudits,
        Self::QuantumKeyDistribution,
        Self::CryptanalysisAndPqc,
        Self::AdvancedQuantumComputing,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::BasicQuantumIntro => "0. Basic Quantum Mechanics & Wave-Particle Intro",
            Self::FoundationsOpenSystems => "1. Qubit Foundations & Open-System Trajectories",
            Self::HighDimensionalQudits => "2. High-Dimensional Optical Qudits (d >= 2)",
            Self::QuantumKeyDistribution => "3. Photonic Quantum Key Distribution & Bell Security",
            Self::CryptanalysisAndPqc => "4. Quantum Cryptanalysis & NIST Post-Quantum Crypto",
            Self::AdvancedQuantumComputing => {
                "5. Advanced Fault-Tolerant & Variational Computing"
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ModuleId {
    I1DoubleSlit,
    I2MachZehnder,
    I3SternGerlach,
    I4QuantumZeno,
    Q1BlochSingleQubit,
    Q2CircuitEntanglement,
    Q3Teleportation,
    Q4OpenSystemsQec,
    Q5EntanglementSwapping,
    D1QuditWeylLab,
    D2QuditTeleportation,
    K1Bb84Qkd,
    K2E91BellChsh,
    K3B92NonOrthogonal,
    C1ShorRsa,
    C2GroverSymmetric,
    C3MlKemLattice,
    C4PqcSignatures,
    A1SurfaceCode,
    A2VqeHydrogen,
    A3QaoaMaxCut,
    A4MagicStateFactory,
    A5GateTeleportation,
}

impl ModuleId {
    pub const ALL: [Self; 23] = [
        Self::I1DoubleSlit,
        Self::I2MachZehnder,
        Self::I3SternGerlach,
        Self::I4QuantumZeno,
        Self::Q1BlochSingleQubit,
        Self::Q2CircuitEntanglement,
        Self::Q3Teleportation,
        Self::Q4OpenSystemsQec,
        Self::Q5EntanglementSwapping,
        Self::D1QuditWeylLab,
        Self::D2QuditTeleportation,
        Self::K1Bb84Qkd,
        Self::K2E91BellChsh,
        Self::K3B92NonOrthogonal,
        Self::C1ShorRsa,
        Self::C2GroverSymmetric,
        Self::C3MlKemLattice,
        Self::C4PqcSignatures,
        Self::A1SurfaceCode,
        Self::A2VqeHydrogen,
        Self::A3QaoaMaxCut,
        Self::A4MagicStateFactory,
        Self::A5GateTeleportation,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Self::I1DoubleSlit => "I1",
            Self::I2MachZehnder => "I2",
            Self::I3SternGerlach => "I3",
            Self::I4QuantumZeno => "I4",
            Self::Q1BlochSingleQubit => "Q1",
            Self::Q2CircuitEntanglement => "Q2",
            Self::Q3Teleportation => "Q3",
            Self::Q4OpenSystemsQec => "Q4",
            Self::Q5EntanglementSwapping => "Q5",
            Self::D1QuditWeylLab => "D1",
            Self::D2QuditTeleportation => "D2",
            Self::K1Bb84Qkd => "K1",
            Self::K2E91BellChsh => "K2",
            Self::K3B92NonOrthogonal => "K3",
            Self::C1ShorRsa => "C1",
            Self::C2GroverSymmetric => "C2",
            Self::C3MlKemLattice => "C3",
            Self::C4PqcSignatures => "C4",
            Self::A1SurfaceCode => "A1",
            Self::A2VqeHydrogen => "A2",
            Self::A3QaoaMaxCut => "A3",
            Self::A4MagicStateFactory => "A4",
            Self::A5GateTeleportation => "A5",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::I1DoubleSlit => "i1-double-slit",
            Self::I2MachZehnder => "i2-mach-zehnder",
            Self::I3SternGerlach => "i3-stern-gerlach",
            Self::I4QuantumZeno => "i4-quantum-zeno",
            Self::Q1BlochSingleQubit => "q1-bloch",
            Self::Q2CircuitEntanglement => "q2-circuit",
            Self::Q3Teleportation => "q3-teleport",
            Self::Q4OpenSystemsQec => "q4-decoherence-qec",
            Self::Q5EntanglementSwapping => "q5-entanglement-swapping",
            Self::D1QuditWeylLab => "d1-qudit",
            Self::D2QuditTeleportation => "d2-qudit-teleport",
            Self::K1Bb84Qkd => "k1-bb84",
            Self::K2E91BellChsh => "k2-e91",
            Self::K3B92NonOrthogonal => "k3-b92",
            Self::C1ShorRsa => "c1-shor",
            Self::C2GroverSymmetric => "c2-grover",
            Self::C3MlKemLattice => "c3-ml-kem",
            Self::C4PqcSignatures => "c4-pqc-sig",
            Self::A1SurfaceCode => "a1-surface-code",
            Self::A2VqeHydrogen => "a2-vqe-h2",
            Self::A3QaoaMaxCut => "a3-qaoa-maxcut",
            Self::A4MagicStateFactory => "a4-magic-state",
            Self::A5GateTeleportation => "a5-gate-teleportation",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        let norm = s.trim().to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|m| m.slug() == norm || m.code().eq_ignore_ascii_case(&norm))
    }

    pub fn group(self) -> CurriculumGroup {
        match self {
            Self::I1DoubleSlit
            | Self::I2MachZehnder
            | Self::I3SternGerlach
            | Self::I4QuantumZeno => CurriculumGroup::BasicQuantumIntro,
            Self::Q1BlochSingleQubit
            | Self::Q2CircuitEntanglement
            | Self::Q3Teleportation
            | Self::Q4OpenSystemsQec
            | Self::Q5EntanglementSwapping => CurriculumGroup::FoundationsOpenSystems,
            Self::D1QuditWeylLab | Self::D2QuditTeleportation => CurriculumGroup::HighDimensionalQudits,
            Self::K1Bb84Qkd | Self::K2E91BellChsh | Self::K3B92NonOrthogonal => {
                CurriculumGroup::QuantumKeyDistribution
            }
            Self::C1ShorRsa
            | Self::C2GroverSymmetric
            | Self::C3MlKemLattice
            | Self::C4PqcSignatures => CurriculumGroup::CryptanalysisAndPqc,
            Self::A1SurfaceCode
            | Self::A2VqeHydrogen
            | Self::A3QaoaMaxCut
            | Self::A4MagicStateFactory
            | Self::A5GateTeleportation => CurriculumGroup::AdvancedQuantumComputing,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::I1DoubleSlit => "Double-Slit Single-Quantum Build-Up & Which-Path Decoherence",
            Self::I2MachZehnder => "Mach-Zehnder Interferometer & Interaction-Free Detection",
            Self::I3SternGerlach => "Stern-Gerlach Sequential Spin-1/2 Beam (Z -> theta -> Z)",
            Self::I4QuantumZeno => "Malus Polarizer Stack & Quantum Zeno Effect",
            Self::Q1BlochSingleQubit => "Rabi Precession & Quantum-Jump Bloch Cloud",
            Self::Q2CircuitEntanglement => "Multi-Qubit Stochastic Gate-Noise & Shot Stream",
            Self::Q3Teleportation => "Continuous EPR Teleportation Photon-Pair Stream",
            Self::Q4OpenSystemsQec => "Lindblad Walker Cloud & 3-Qubit Stabilizer Loop",
            Self::Q5EntanglementSwapping => "Entanglement Swapping & Linear-Optical Quantum Repeater",
            Self::D1QuditWeylLab => "High-Dimensional Qudit (d = 2..8) Waveguide Stream",
            Self::D2QuditTeleportation => "d-Dimensional Weyl-Heisenberg Qudit Teleportation",
            Self::K1Bb84Qkd => "Live BB84 Photonic Fiber Stream (Alice -> Eve -> Bob)",
            Self::K2E91BellChsh => "Live E91 Entangled Singlet Stream & CHSH Bell Drift",
            Self::K3B92NonOrthogonal => "Live B92 Non-Orthogonal Stream & POVM Sorter",
            Self::C1ShorRsa => "Shot-by-Shot Shor Inverse-QFT Resonance Stream",
            Self::C2GroverSymmetric => "Continuous Grover Rotor & Amplitude-Amplification Wave",
            Self::C3MlKemLattice => "Live NIST FIPS 203 ML-KEM Encapsulation & Lattice Cloud",
            Self::C4PqcSignatures => "FIPS 204 Rejection-Sampling Fountain & Merkle Stream",
            Self::A1SurfaceCode => "2D Rotated Surface Code (d = 3, 5, 7) & Threshold Crossover",
            Self::A2VqeHydrogen => "Variational Quantum Eigensolver (VQE) Molecular H2 Trajectory",
            Self::A3QaoaMaxCut => "QAOA Combinatorial Max-Cut & Neutral-Atom Rydberg Graph",
            Self::A4MagicStateFactory => "Bravyi-Kitaev 15-to-1 Magic-State Distillation Factory",
            Self::A5GateTeleportation => "Gottesman-Chuang Magic-State T-Gate Teleportation",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Self::I1DoubleSlit => {
                "Watch individual quanta land dot-by-dot on a phosphor screen to sculpt Young interference fringes, and dial up which-path detector coupling to watch fringes melt into two classical mounds."
            }
            Self::I2MachZehnder => {
                "Watch single photons split across upper and lower interferometer arms with phase phi; toggle an arm absorber to observe Elitzur-Vaidman interaction-free dark-port clicks."
            }
            Self::I3SternGerlach => {
                "Stream spin-1/2 silver atoms through three sequential magnetic gradients (Z -> theta -> Z) to see non-commuting X/theta filtration revive a blocked -Z spin state."
            }
            Self::I4QuantumZeno => {
                "Send horizontally polarized photons through N intermediate polarizers spanning 0 to 90 deg; watch frequent projective measurements freeze polarization rotation via cos^(2N)(pi / 2N)."
            }
            Self::Q1BlochSingleQubit => {
                "Watch 64 Bloch trajectories precess around (Omega_x, Omega_y, Delta_z) while undergoing stochastic Poissonian spontaneous-emission jumps to |0>."
            }
            Self::Q2CircuitEntanglement => {
                "Watch photonic wavepackets traverse Bell, GHZ, or QFT circuits with stochastic gate-fault sparks as empirical Born histograms converge."
            }
            Self::Q3Teleportation => {
                "Watch SPDC entangled pairs fly to Alice and Bob, trigger 2-bit classical feed-forward packets, and populate Bob's conditional Bloch cloud."
            }
            Self::Q4OpenSystemsQec => {
                "Watch open-system states decay inside the Bloch ball while periodic 3-qubit stabilizer syndromes detect and heal single-qubit physical errors."
            }
            Self::Q5EntanglementSwapping => {
                "Watch N_hops + 1 independent SPDC sources fire entangled photon pairs into N_hops intermediate Bell-state analyzers, heralding end-to-end entanglement with optional 50% linear-optical post-selection."
            }
            Self::D1QuditWeylLab => {
                "Watch d-level orbital-angular-momentum pulses propagate across d optical rails through F_d + CSUM couplers to build live |j, j+a> coincidences."
            }
            Self::D2QuditTeleportation => {
                "Watch a d-level photonic superposition (d = 2..8) undergo a d^2-outcome generalized Bell projection (m, n), relay 2*log2(d) classical bits, and reconstruct via Weyl Z_d^m X_d^n above the 2/(d+1) classical limit."
            }
            Self::K1Bb84Qkd => {
                "Watch polarized single photons pass Eve's beam-splitter probe into Bob's detector while the live sifted bit tape scrolls and QBER(t) converges."
            }
            Self::K2E91BellChsh => {
                "Watch singlet |Psi-> pairs enter rotating 3x3 analyzers as empirical CHSH |S(t)| converges toward Tsirelson's 2*sqrt(2) = 2.828 ceiling."
            }
            Self::K3B92NonOrthogonal => {
                "Watch non-orthogonal |psi_0> and |psi_1> pulses sort into Bob's conclusive POVM ports vs. inconclusive erasure dump."
            }
            Self::C1ShorRsa => {
                "Watch independent order-finding measurement shots build constructive interference spikes at j*2^m/r to lock period r and factor N = p*q."
            }
            Self::C2GroverSymmetric => {
                "Watch the statevector rotate continuously in the (|psi_perp>, |w>) plane while candidate cipher-key probabilities surge and over-rotate."
            }
            Self::C3MlKemLattice => {
                "Watch continuous R_3329 KeyGen-Encaps-Decaps handshakes stream |e_dec(t)| against q/4 = 832 and scatter noisy t = A*s + e lattice vectors."
            }
            Self::C4PqcSignatures => {
                "Watch candidate z = y + c*s vectors rain through the ||z||_inf < gamma_1 - beta gate while accepted blocks extend a SHA-256 Merkle chain."
            }
            Self::A1SurfaceCode => {
                "Watch stochastic X/Z Pauli errors strike a d x d rotated surface-code lattice, light up plaquette syndromes, and demonstrate logical survival crossover across p_th ~ 1.0%."
            }
            Self::A2VqeHydrogen => {
                "Watch a shot-noise SPSA variational optimizer descend the 2-qubit H2 molecular energy landscape E(theta) across bond lengths R = 0.3..2.4 Angstrom toward chemical accuracy."
            }
            Self::A3QaoaMaxCut => {
                "Watch alternating cost/mixer QAOA layers (p = 1..6) pulse across an 8-node Rydberg atom graph, concentrating bitstring shots onto maximum-cut partitions."
            }
            Self::A4MagicStateFactory => {
                "Watch noisy raw |T> ancillas enter 15-to-1 Bravyi-Kitaev Reed-Muller factories, post-selecting clean stabilizer blocks to achieve cubic 35*p^3 output error suppression."
            }
            Self::A5GateTeleportation => {
                "Watch non-Clifford T = diag(1, exp(i*pi/4)) gates execute fault-tolerantly via |T> ancilla injection, CNOT coupling, Z-measurement, and conditional Clifford S^m feed-forward across chained octant steps."
            }
        }
    }

    pub fn key_equation(self) -> &'static str {
        match self {
            Self::I1DoubleSlit => {
                "I(x) = I_1(x) + I_2(x) + 2*(1 - eta_which_path)*sqrt(I_1 I_2)*cos(2*pi*d*x / lambda*L)"
            }
            Self::I2MachZehnder => {
                "No blocker: P(D1) = cos^2(phi/2), P(D2) = sin^2(phi/2)  |  Arm blocked (phi=0): P_absorb=50%, P(D1)=25%, P_IFM(D2)=25%"
            }
            Self::I3SternGerlach => {
                "|+z> -> SG(theta) -> |+theta> (prob cos^2(theta/2)) -> SG(z) -> |-z> revived with prob (1/4)*sin^2(theta)"
            }
            Self::I4QuantumZeno => {
                "P_survive(0 -> 90 deg in N stages) = [cos^2(pi / (2*N))]^N  -->  1.000 as N -> infinity"
            }
            Self::Q1BlochSingleQubit => {
                "dr/dt = Omega x r - gamma_perp*(rx, ry, 0) + Poisson_Jumps(|0>)   |   r = (<X>, <Y>, <Z>)"
            }
            Self::Q2CircuitEntanglement => {
                "rho_A = Tr_B(|psi><psi|),   S(rho_A) = - sum_i lambda_i log2(lambda_i)  [ebits]"
            }
            Self::Q3Teleportation => {
                "|psi>_0 (x) |Phi^+>_{12}  -->  1/2 sum_{m0,m1} |Bell_{m0,m1}>_{01} (x) (X^{m1} Z^{m0})^dagger |psi>_2"
            }
            Self::Q4OpenSystemsQec => {
                "rho' = sum_k E_k rho E_k^dagger,   P_logical_fail = 3*p^2*(1 - p) + p^3 < p"
            }
            Self::Q5EntanglementSwapping => {
                "w_end = (1 - 4p/3)^(N_hops + 1),   F_Bell = (1 + 3*w_end)/4,   P_herald(LO-BSM) = (1/2)^N_hops"
            }
            Self::D1QuditWeylLab => {
                "Z_d X_d = exp(2*pi*i/d) X_d Z_d,   |Phi_d> = (1/sqrt(d)) sum_{j=0}^{d-1} |j, j>,   S = log2(d) ebits"
            }
            Self::D2QuditTeleportation => {
                "|psi_d> (x) |Phi_d> -> Bell_d(m,n) -> Z_d^m X_d^n:  F_avg = 1 - p*(d-1)/d  (Classical = 2/(d+1))"
            }
            Self::K1Bb84Qkd => {
                "QBER_Eve = 0.25 * p_Eve,   R_asymptotic = max(0, 1 - 2*h2(QBER))  (secure for QBER < 11.0%)"
            }
            Self::K2E91BellChsh => {
                "S = E(a1,b1) - E(a1,b3) + E(a3,b1) + E(a3,b3),   |S|_local <= 2.0 < |S|_QM = 2*sqrt(2) = 2.8284"
            }
            Self::K3B92NonOrthogonal => {
                "|<psi_0|psi_1>| = cos(theta),   P_sift = 0.5*sin^2(theta),   P_USD_max = 1 - cos(theta)"
            }
            Self::C1ShorRsa => {
                "a^r == 1 (mod N),   r even & a^(r/2) != -1 (mod N)  ==>  gcd(a^(r/2) +/- 1, N) in {p, q}"
            }
            Self::C2GroverSymmetric => {
                "G = (2|s><s| - I) O_f,   P_target(k) = sin^2((2k + 1)*arcsin(sqrt(M / N))),   k_opt ~ (pi/4)*sqrt(N/M)"
            }
            Self::C3MlKemLattice => {
                "t = A*s + e (mod 3329, X^n + 1),   w = v - s^T*u = round(q/2)*m + e_dec,   |e_dec| < 832"
            }
            Self::C4PqcSignatures => {
                "z = y + c*s:  accept iff ||z||_inf < gamma_1 - beta   |   Merkle root = SHA256(H_left || H_right)"
            }
            Self::A1SurfaceCode => {
                "[[d^2, 1, d]] Rotated Surface Code:  P_L ~ 0.1 * (p / p_th)^((d + 1)/2),   p_th ~ 1.0%"
            }
            Self::A2VqeHydrogen => {
                "H_{H2}(R) = g0*I + g1*Z0 + g2*Z1 + g3*Z0Z1 + g4*(X0X1 + Y0Y1),   E(theta) >= E_FCI(R)"
            }
            Self::A3QaoaMaxCut => {
                "|gamma, beta> = prod_{l=1}^p exp(-i*beta_l sum X_i) exp(-i*gamma_l C_MaxCut) |+>^{n}"
            }
            Self::A4MagicStateFactory => {
                "[[15, 1, 3]] Reed-Muller T-Distillation:  p_out ~ 35 * p_in^3,   P_accept ~ (1 - 15*p_in)"
            }
            Self::A5GateTeleportation => {
                "CNOT(|psi>|T>) -> measure Z_anc = m in {0,1} -> Clifford S^m  ==>  exp(i*m*pi/4) T|psi>"
            }
        }
    }

    pub fn citation(self) -> (&'static str, &'static str) {
        match self {
            Self::I1DoubleSlit => (
                "Bach, Pope, Liou & Batelaan (2013) Controlled double-slit electron diffraction, New J. Phys. 15, 033018",
                "https://arxiv.org/abs/1210.6243",
            ),
            Self::I2MachZehnder => (
                "Elitzur & Vaidman (1993) Quantum mechanical interaction-free measurements, Found. Phys. 23, 987",
                "https://arxiv.org/abs/hep-th/9305002",
            ),
            Self::I3SternGerlach => (
                "Gerlach & Stern (1922) Experimental proof of directional quantization in a magnetic field, arXiv:2301.11343",
                "https://arxiv.org/abs/2301.11343",
            ),
            Self::I4QuantumZeno => (
                "Misra & Sudarshan (1977) The Zeno's paradox in quantum theory, J. Math. Phys. 18, 756",
                "https://pubs.aip.org/aip/jmp/article/18/4/756/225634/The-Zeno-s-paradox-in-quantum-theory",
            ),
            Self::Q1BlochSingleQubit => (
                "Feynman, Vernon & Hellwarth (1957) Geometrical Representation of the Schrodinger Equation, J. Appl. Phys. 28, 49",
                "https://pubs.aip.org/aip/jap/article/28/1/49/161271/Geometrical-Representation-of-the-Schrodinger",
            ),
            Self::Q2CircuitEntanglement => (
                "Cross et al. (2017) Open Quantum Assembly Language (OpenQASM 2.0), arXiv:1707.03429",
                "https://arxiv.org/abs/1707.03429",
            ),
            Self::Q3Teleportation => (
                "Bennett et al. (1993) Teleporting an unknown quantum state, Phys. Rev. Lett. 70, 1895",
                "https://link.aps.org/doi/10.1103/PhysRevLett.70.1895",
            ),
            Self::Q4OpenSystemsQec => (
                "Shor (1995) Scheme for reducing decoherence in quantum computer memory, Phys. Rev. A 52, R2493",
                "https://link.aps.org/doi/10.1103/PhysRevA.52.R2493",
            ),
            Self::Q5EntanglementSwapping => (
                "Zukowski, Zeilinger, Horne & Ekert (1993) Event-ready-detectors Bell experiment via entanglement swapping, Phys. Rev. Lett. 71, 4287",
                "https://link.aps.org/doi/10.1103/PhysRevLett.71.4287",
            ),
            Self::D1QuditWeylLab => (
                "Wang, Hu, Sanders & Kais (2020) Qudits and High-Dimensional Quantum Computing, Front. Phys. 8:589504",
                "https://www.frontiersin.org/journals/physics/articles/10.3389/fphy.2020.589504/full",
            ),
            Self::D2QuditTeleportation => (
                "Luo et al. (2019) Quantum Teleportation in High Dimensions, Phys. Rev. Lett. 123, 070505",
                "https://link.aps.org/doi/10.1103/PhysRevLett.123.070505",
            ),
            Self::K1Bb84Qkd => (
                "Bennett & Brassard (1984) Quantum cryptography: Public key distribution and coin tossing, arXiv:2003.06557",
                "https://arxiv.org/abs/2003.06557",
            ),
            Self::K2E91BellChsh => (
                "Ekert (1991) Quantum cryptography based on Bell's theorem, Phys. Rev. Lett. 67, 661",
                "https://link.aps.org/doi/10.1103/PhysRevLett.67.661",
            ),
            Self::K3B92NonOrthogonal => (
                "Bennett (1992) Quantum cryptography using any two nonorthogonal states, Phys. Rev. Lett. 68, 3121",
                "https://link.aps.org/doi/10.1103/PhysRevLett.68.3121",
            ),
            Self::C1ShorRsa => (
                "Shor (1997) Polynomial-Time Algorithms for Prime Factorization and Discrete Logarithms, SIAM J. Comput. 26(5), 1484",
                "https://epubs.siam.org/doi/10.1137/S0097539795293172",
            ),
            Self::C2GroverSymmetric => (
                "Grover (1996) A fast quantum mechanical algorithm for database search, STOC '96 / arXiv:quant-ph/9605043",
                "https://arxiv.org/abs/quant-ph/9605043",
            ),
            Self::C3MlKemLattice => (
                "NIST FIPS 203 (2024) Module-Lattice-Based Key-Encapsulation Mechanism Standard (ML-KEM)",
                "https://csrc.nist.gov/pubs/fips/203/final",
            ),
            Self::C4PqcSignatures => (
                "NIST FIPS 204 (2024) Module-Lattice-Based Digital Signature Standard (ML-DSA)",
                "https://csrc.nist.gov/pubs/fips/204/final",
            ),
            Self::A1SurfaceCode => (
                "Fowler, Mariantoni, Martinis & Cleland (2012) Surface codes: Towards practical large-scale quantum computation, Phys. Rev. A 86, 032324",
                "https://link.aps.org/doi/10.1103/PhysRevA.86.032324",
            ),
            Self::A2VqeHydrogen => (
                "Peruzzo, McClean, Shadbolt et al. (2014) A variational eigenvalue solver on a photonic quantum processor, Nat. Commun. 5, 4213",
                "https://pmc.ncbi.nlm.nih.gov/articles/PMC4124861/",
            ),
            Self::A3QaoaMaxCut => (
                "Farhi, Goldstone & Gutmann (2014) A Quantum Approximate Optimization Algorithm, arXiv:1411.4028",
                "https://arxiv.org/abs/1411.4028",
            ),
            Self::A4MagicStateFactory => (
                "Bravyi & Kitaev (2005) Universal quantum computation with ideal Clifford gates and noisy ancillas, Phys. Rev. A 71, 022316",
                "https://link.aps.org/doi/10.1103/PhysRevA.71.022316",
            ),
            Self::A5GateTeleportation => (
                "Gottesman & Chuang (1999) Quantum Teleportation is a Universal Computational Primitive, arXiv:quant-ph/9908010",
                "https://arxiv.org/abs/quant-ph/9908010",
            ),
        }
    }
}
