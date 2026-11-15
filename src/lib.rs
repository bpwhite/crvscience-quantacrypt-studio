#![forbid(unsafe_code)]

//! QuantaCrypt Studio (`quantacrypt-studio` v2.0.0)
//! Pure safe-Rust passive stochastic quantum trajectory, open-system Kraus/jump decoherence,
//! high-dimensional qudit waveguide, photonic QKD (BB84, E91, B92), quantum cryptanalysis
//! (Shor, Grover), and NIST FIPS 203/204 post-quantum cryptography simulator.

pub mod complex;
pub mod cryptanalysis;
pub mod curriculum;
pub mod density;
pub mod gui;
pub mod plotter;
pub mod pqc_lattice;
pub mod qkd;
pub mod qudit;
pub mod rng_sha256;
pub mod statevector;
pub mod stochastic_sim;
pub mod workset;
