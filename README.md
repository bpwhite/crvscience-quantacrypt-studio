# QuantaCrypt Studio (`quantacrypt-studio` v2.2.0)

![QuantaCrypt Studio — 4-Qubit GHZ Circuit & Born-Rule Shot Observatory](validation/headless_screenshot.png)

**QuantaCrypt Studio (`v2.2.0`)** is a zero-unsafe (`#![forbid(unsafe_code)]`), native safe-Rust **23-module passive stochastic quantum & cryptography observatory** (`eframe` / `egui` `0.31`) and headless CLI (`clap`) spanning foundational wave-particle experiments through fault-tolerant surface codes, variational quantum eigensolvers, QAOA, and NIST post-quantum cryptography.

## 23 Autonomous Stochastic Modules Across 6 Groups

### 0. Basic Quantum Mechanics & Wave-Particle Intro
- **`I1` — Double-Slit Single-Quantum Build-Up & Which-Path Decoherence** ([Bach et al. 2013](https://arxiv.org/abs/1210.6243))
- **`I2` — Mach-Zehnder Interferometer & Interaction-Free Detection** ([Elitzur & Vaidman 1993](https://arxiv.org/abs/hep-th/9305002))
- **`I3` — Stern-Gerlach Sequential Spin-1/2 Beam (`Z -> theta -> Z`)** ([Gerlach & Stern 1922](https://arxiv.org/abs/2301.11343))
- **`I4` — Malus Polarizer Stack & Quantum Zeno Effect** ([Misra & Sudarshan 1977](https://pubs.aip.org/aip/jmp/article/18/4/756/225634/The-Zeno-s-paradox-in-quantum-theory))

### 1. Qubit Foundations & Open-System Trajectories
- **`Q1` — Rabi Precession & Quantum-Jump Bloch Cloud** ([Feynman, Vernon & Hellwarth 1957](https://pubs.aip.org/aip/jap/article/28/1/49/161271/Geometrical-Representation-of-the-Schrodinger))
- **`Q2` — Multi-Qubit Stochastic Gate-Noise & Shot Stream** ([Cross et al. 2017, OpenQASM 2.0](https://arxiv.org/abs/1707.03429))
- **`Q3` — Continuous EPR Teleportation Photon-Pair Stream** ([Bennett et al. 1993](https://link.aps.org/doi/10.1103/PhysRevLett.70.1895))
- **`Q4` — Lindblad Walker Cloud & 3-Qubit Stabilizer Loop** ([Shor 1995](https://link.aps.org/doi/10.1103/PhysRevA.52.R2493))
- **`Q5` — Entanglement Swapping & Linear-Optical Quantum Repeater** ([Zukowski et al. 1993](https://link.aps.org/doi/10.1103/PhysRevLett.71.4287); [Bouwmeester et al. 1997](https://arxiv.org/abs/1901.11004))

### 2. High-Dimensional Optical Qudits
- **`D1` — High-Dimensional Qudit (`d = 2..8`) Waveguide Stream** ([Wang et al. 2020](https://www.frontiersin.org/journals/physics/articles/10.3389/fphy.2020.589504/full))
- **`D2` — `d`-Dimensional Weyl-Heisenberg Qudit Teleportation** ([Luo et al. 2019](https://link.aps.org/doi/10.1103/PhysRevLett.123.070505))

### 3. Photonic Quantum Key Distribution & Bell Security
- **`K1` — Live BB84 Photonic Fiber Stream** ([Bennett & Brassard 1984](https://arxiv.org/abs/2003.06557))
- **`K2` — Live E91 Entangled Singlet Stream & CHSH Bell Drift** ([Ekert 1991](https://link.aps.org/doi/10.1103/PhysRevLett.67.661))
- **`K3` — Live B92 Non-Orthogonal Stream & POVM Sorter** ([Bennett 1992](https://link.aps.org/doi/10.1103/PhysRevLett.68.3121))

### 4. Quantum Cryptanalysis & NIST Post-Quantum Cryptography
- **`C1` — Shot-by-Shot Shor Inverse-QFT Resonance Stream** ([Shor 1997](https://epubs.siam.org/doi/10.1137/S0097539795293172))
- **`C2` — Continuous Grover Rotor & Amplitude-Amplification Wave** ([Grover 1996](https://arxiv.org/abs/quant-ph/9605043))
- **`C3` — Live NIST FIPS 203 ML-KEM Encapsulation & Lattice Cloud** ([NIST FIPS 203](https://csrc.nist.gov/pubs/fips/203/final))
- **`C4` — FIPS 204 Rejection-Sampling Fountain & Merkle Stream** ([NIST FIPS 204](https://csrc.nist.gov/pubs/fips/204/final); [NIST SP 800-208](https://csrc.nist.gov/pubs/sp/800/208/final))

### 5. Advanced Fault-Tolerant & Variational Computing
- **`A1` — 2D Rotated Surface Code (`d = 3, 5, 7`) & Threshold Crossover** ([Fowler et al. 2012](https://link.aps.org/doi/10.1103/PhysRevA.86.032324))
- **`A2` — Variational Quantum Eigensolver (`VQE`) Molecular `H2` Trajectory** ([Peruzzo et al. 2014](https://pmc.ncbi.nlm.nih.gov/articles/PMC4124861/))
- **`A3` — QAOA Combinatorial Max-Cut & Neutral-Atom Rydberg Graph** ([Farhi, Goldstone & Gutmann 2014](https://arxiv.org/abs/1411.4028))
- **`A4` — Bravyi-Kitaev `15-to-1` Magic-State Distillation Factory** ([Bravyi & Kitaev 2005](https://link.aps.org/doi/10.1103/PhysRevA.71.022316))
- **`A5` — Gottesman-Chuang Magic-State `T`-Gate Teleportation** ([Gottesman & Chuang 1999](https://arxiv.org/abs/quant-ph/9908010))

## Build & Run

```bash
cargo test
cargo run --release
cargo run --release -- --module i1-double-slit
cargo run --release -- --module a1-surface-code
```

## Bundled Documentation & Browser Observatory

- **Illustrated User & Physics Guide (PDF)**: `docs/QuantaCrypt_Studio_v2_2_0_Guide.pdf`
- **Standalone 60 FPS Browser Observatory**: `web/quantacrypt_studio_preview.html`
- **OpenQASM 2.0 Example Circuits**: `examples/*.qasm`
