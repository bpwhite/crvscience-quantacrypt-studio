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
