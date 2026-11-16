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

//! End-to-end integration tests for QuantaCrypt Studio (`v2.2.0`, 23 modules).

use quantacrypt_studio::cryptanalysis::{run_grover_key_search, run_shor_cryptanalysis};
use quantacrypt_studio::curriculum::ModuleId;
use quantacrypt_studio::density::{
    simulate_decoherence_trajectory, simulate_three_qubit_qec, NoiseChannelKind, QecCodeKind,
};
use quantacrypt_studio::pqc_lattice::{run_ml_kem_lab, run_pqc_signature_lab, MlKemProfile};
use quantacrypt_studio::qkd::{simulate_b92, simulate_bb84, simulate_e91};
use quantacrypt_studio::qudit::{run_qudit_lab, QuditPreset};
use quantacrypt_studio::rng_sha256::sha256_hex;
use quantacrypt_studio::statevector::{simulate_teleportation, QuantumCircuit, Statevector};
use quantacrypt_studio::stochastic_sim::StochasticArena;
use quantacrypt_studio::workset::{PortableWorksetEnvelope, WorksetRepository, WorksetState};

#[test]
fn sha256_matches_nist_fips_180_4_vectors() {
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn stochastic_arena_advances_all_twenty_three_modules() {
    let mut ws = WorksetState::default();
    assert_eq!(ModuleId::ALL.len(), 23);
    for m in ModuleId::ALL {
        ws.active_module = m;
        let mut arena = StochasticArena::new(2026, m);
        for _ in 0..30 {
            arena.step(0.033, &ws);
        }
        assert!(arena.sim_time > 0.5);
        assert!(arena.total_events > 0, "Expected stochastic events in {:?}", m);
    }
}

#[test]
fn bell_ghz_entanglement_and_openqasm_roundtrip() {
    let bell = QuantumCircuit::preset_bell_phi_plus();
    let sv_bell = Statevector::execute_circuit(&bell, None, 101);
    let ebits = sv_bell.bipartite_entanglement_ebits(1);
    assert!((ebits - 1.0).abs() < 1e-6);

    let qasm = bell.to_openqasm();
    let parsed = QuantumCircuit::from_openqasm(&qasm).expect("valid OpenQASM 2.0");
    let sv_parsed = Statevector::execute_circuit(&parsed, None, 101);
    for (a, b) in sv_bell.amps.iter().zip(sv_parsed.amps.iter()) {
        assert!(a.approx_eq(*b, 1e-6));
    }

    let ghz4 = QuantumCircuit::preset_ghz(4);
    let sv_ghz = Statevector::execute_circuit(&ghz4, None, 202);
    assert!((sv_ghz.bipartite_entanglement_ebits(2) - 1.0).abs() < 1e-6);
}

#[test]
fn noiseless_teleportation_recovers_unit_fidelity() {
    let rep = simulate_teleportation(1.23, -0.77, 0.0, 999);
    assert!((rep.state_fidelity - 1.0).abs() < 1e-9);
}

#[test]
fn kraus_trace_preservation_and_three_qubit_qec_recovery() {
    let (rho_final, hist) = simulate_decoherence_trajectory(
        std::f64::consts::FRAC_PI_2,
        0.3,
        NoiseChannelKind::Depolarizing,
        0.15,
        20,
    );
    assert!((rho_final.trace().re - 1.0).abs() < 1e-9);
    assert!(hist.last().unwrap().purity < 0.60);

    for fault_q in [None, Some(0), Some(1), Some(2)] {
        let bit_qec = simulate_three_qubit_qec(QecCodeKind::ThreeQubitBitFlip, 1.1, 0.6, 0.05, fault_q);
        assert!(bit_qec.single_fault_recovered_exact);
        let phase_qec = simulate_three_qubit_qec(QecCodeKind::ThreeQubitPhaseFlip, 1.1, 0.6, 0.05, fault_q);
        assert!(phase_qec.single_fault_recovered_exact);
    }
}

#[test]
fn high_dimensional_qudit_weyl_algebra_and_ebits() {
    for d in [2usize, 3, 4, 6, 8] {
        let rep = run_qudit_lab(d, QuditPreset::BellMaximallyEntangledPair, 1, 2);
        assert!(rep.weyl_residual < 1e-11);
        assert!(rep.qft_is_unitary);
        assert!((rep.entanglement_ebits - (d as f64).log2()).abs() < 1e-5);
    }
}

#[test]
fn qkd_bb84_e91_b92_security_regimes() {
    let clean_bb84 = simulate_bb84(2048, 0.0, 0.01, 42);
    assert!(clean_bb84.channel_secure && clean_bb84.keys_identical);
    let eve_bb84 = simulate_bb84(2048, 0.80, 0.01, 42);
    assert!(!eve_bb84.channel_secure);

    let clean_e91 = simulate_e91(6000, 0.0, 0.99, 77);
    assert!(clean_e91.bell_violated && clean_e91.chsh_s_observed > 2.55);

    let b92 = simulate_b92(4000, 45.0, 0.0, 19);
    assert!(b92.observed_qber < 1e-9 && b92.sifted_conclusive_len > 700);
}

#[test]
fn shor_factors_semiprimes_and_grover_amplifies_key() {
    for (n, a) in [(15, 2), (21, 2), (35, 2), (77, 2)] {
        let rep = run_shor_cryptanalysis(n, a, 8);
        let (p, q) = rep.recovered_factors.expect("Shor should recover factors");
        assert_eq!(p * q, n);
    }
    let grover_probe = run_grover_key_search(7, 89, 1);
    let grover_opt = run_grover_key_search(7, 89, grover_probe.optimal_iterations);
    assert!(grover_opt.target_success_prob > 0.96);
}

#[test]
fn ml_kem_fips_203_and_signature_rejection_sampling() {
    for prof in MlKemProfile::ALL {
        let rep = run_ml_kem_lab(prof, prof.default_eta(), 2026);
        assert!(rep.decapsulation_exact);
    }
    let sig = run_pqc_signature_lab(128, 2, 8, true, 2026);
    assert!(sig.acceptance_rate > 0.10 && sig.merkle_lamport.valid_original && !sig.merkle_lamport.valid_tampered);
}

#[test]
fn sqlite_worksets_and_portable_envelope_tamper_detection() {
    let repo = WorksetRepository::open_in_memory().expect("in-memory SQLite");
    let mut st = WorksetState::default();
    st.active_module = ModuleId::D1QuditWeylLab;
    repo.save_slot("alpha", &st).expect("save slot");
    let loaded = repo.load_slot("alpha").expect("load slot");
    assert_eq!(loaded.active_module, ModuleId::D1QuditWeylLab);

    let mut env = PortableWorksetEnvelope::pack("alpha", &st).expect("pack");
    env.payload_json.push(' ');
    assert!(env.unpack().is_err());
}
