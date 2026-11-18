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
use quantacrypt_studio::icon::app_icon_rgba;
use quantacrypt_studio::pqc_lattice::{run_ml_kem_lab, run_pqc_signature_lab, MlKemProfile};
use quantacrypt_studio::qkd::{simulate_b92, simulate_bb84, simulate_e91};
use quantacrypt_studio::qudit::{run_qudit_lab, QuditPreset, QuditRegister};
use quantacrypt_studio::rng_sha256::sha256_hex;
use quantacrypt_studio::statevector::{simulate_teleportation, GateOp, QuantumCircuit, Statevector};
use quantacrypt_studio::stochastic_sim::{
    double_slit_analytical_bin_probs, double_slit_intensity, StochasticArena,
};
use quantacrypt_studio::workset::{PortableWorksetEnvelope, WorksetRepository, WorksetState};
use std::path::Path;

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

#[test]
fn openqasm_nested_parens_bounds_distinct_qubits_and_full_gate_roundtrip() {
    let nested = "OPENQASM 2.0;\ninclude \"qelib1.inc\";\nqreg q[2];\nid q[0];\nrx((pi/2)) q[1];\ncx q[1],q[0];\n";
    let parsed_nested = QuantumCircuit::from_openqasm(nested).expect("nested parens and id gate should parse");
    assert_eq!(parsed_nested.num_qubits, 2);
    assert_eq!(parsed_nested.ops.len(), 3);

    // Out-of-bounds qreg index must fail instead of silently expanding
    let oob = "OPENQASM 2.0;\nqreg q[2];\ncx q[0],q[3];\n";
    assert!(QuantumCircuit::from_openqasm(oob).is_err());

    // Self-target multi-qubit gates must fail
    assert!(QuantumCircuit::from_openqasm("OPENQASM 2.0;\nqreg q[2];\ncx q[0],q[0];\n").is_err());
    assert!(QuantumCircuit::from_openqasm("OPENQASM 2.0;\nqreg q[2];\nswap q[1],q[1];\n").is_err());
    assert!(QuantumCircuit::from_openqasm("OPENQASM 2.0;\nqreg q[3];\nccx q[0],q[1],q[1];\n").is_err());

    // Full gate roundtrip across all 19 GateOp variants
    let circ = QuantumCircuit {
        num_qubits: 3,
        ops: vec![
            GateOp::Id(0),
            GateOp::H(0),
            GateOp::X(1),
            GateOp::Y(2),
            GateOp::Z(0),
            GateOp::S(1),
            GateOp::Sdg(1),
            GateOp::T(2),
            GateOp::Tdg(2),
            GateOp::Rx { target: 0, theta: 0.7 },
            GateOp::Ry { target: 1, theta: 1.1 },
            GateOp::Rz { target: 2, theta: -0.4 },
            GateOp::Phase { target: 0, lambda: 0.9 },
            GateOp::Cnot { control: 0, target: 1 },
            GateOp::Cz { control: 1, target: 2 },
            GateOp::CPhase { control: 0, target: 2, lambda: 0.5 },
            GateOp::Swap(0, 2),
            GateOp::Toffoli { c1: 0, c2: 1, target: 2 },
            GateOp::Measure(2),
        ],
    };
    let qasm = circ.to_openqasm();
    let roundtrip = QuantumCircuit::from_openqasm(&qasm).expect("full gate roundtrip should parse");
    let sv1 = Statevector::execute_circuit(&circ, None, 2026);
    let sv2 = Statevector::execute_circuit(&roundtrip, None, 2026);
    for (a, b) in sv1.amps.iter().zip(sv2.amps.iter()) {
        assert!(a.approx_eq(*b, 1e-5));
    }
}

#[test]
fn q2_density_matrix_full_and_reduced_entanglement_invariants() {
    let ghz4 = QuantumCircuit::preset_ghz(4);
    let sv4 = Statevector::execute_circuit(&ghz4, None, 123);
    let rho_full = sv4.full_density_matrix().expect("4 qubits has 16x16 full rho");
    assert_eq!(rho_full.dim, 16);
    assert!((rho_full.trace().re - 1.0).abs() < 1e-9);
    assert!((rho_full.purity() - 1.0).abs() < 1e-9);
    assert!((rho_full.get(0, 15).norm() - 0.5).abs() < 1e-9);
    let rho_reduced = sv4.reduced_subsystem_rho(2);
    assert_eq!(rho_reduced.dim, 4);
    assert!((rho_reduced.von_neumann_entropy_bits() - 1.0).abs() < 1e-6);

    let bell = QuantumCircuit::preset_bell_phi_plus();
    let sv2 = Statevector::execute_circuit(&bell, None, 123);
    let rho2 = sv2.full_density_matrix().expect("2 qubits has 4x4 full rho");
    assert!((rho2.trace().re - 1.0).abs() < 1e-9);
    assert!((rho2.purity() - 1.0).abs() < 1e-9);
    assert!((sv2.bipartite_entanglement_ebits(1) - 1.0).abs() < 1e-6);

    let sv5 = Statevector::zero_state(5);
    assert!(sv5.full_density_matrix().is_none());
}

#[test]
fn i1_double_slit_analytical_curve_matches_monte_carlo_sampling() {
    let theory = double_slit_analytical_bin_probs(4.5, 0.0, 64);
    assert_eq!(theory.len(), 64);
    let sum: f64 = theory.iter().sum();
    assert!((sum - 1.0).abs() < 1e-12);
    // Constructive interference at x=0 exceeds which-path=1.0 intensity at x=0
    assert!(double_slit_intensity(0.0, 4.5, 0.0) > double_slit_intensity(0.0, 4.5, 1.0));

    let mut ws = WorksetState::default();
    ws.active_module = ModuleId::I1DoubleSlit;
    ws.i1_slit_sep = 4.5;
    ws.i1_which_path = 0.0;
    let mut arena = StochasticArena::new(2026, ModuleId::I1DoubleSlit);
    for _ in 0..800 {
        arena.step(0.05, &ws);
    }
    let total: u64 = arena.i1_bins.iter().sum();
    assert!(total > 1500, "Expected >1500 double-slit hits, got {}", total);
    let l1_diff: f64 = arena
        .i1_bins
        .iter()
        .zip(theory.iter())
        .map(|(&cnt, &p)| ((cnt as f64) / (total as f64) - p).abs())
        .sum();
    assert!(l1_diff < 0.25, "Analytical vs empirical L1 discrepancy too high: {}", l1_diff);
}

#[test]
fn k1_bb84_sifted_bit_tape_records_alice_eve_bob_invariants() {
    let mut ws = WorksetState::default();
    ws.active_module = ModuleId::K1Bb84Qkd;
    ws.k1_eve_rate = 0.5;
    ws.k1_channel_noise = 0.0;
    let mut arena = StochasticArena::new(2026, ModuleId::K1Bb84Qkd);
    for _ in 0..120 {
        arena.step(0.05, &ws);
    }
    assert!(!arena.k1_bit_tape.is_empty());
    let mut saw_eve = false;
    let mut saw_clean = false;
    for entry in &arena.k1_bit_tape {
        assert!(entry.sifted_keep);
        assert!(entry.alice_basis == '+' || entry.alice_basis == 'x');
        assert_eq!(entry.alice_basis, entry.bob_basis);
        if entry.eve_intercepted {
            saw_eve = true;
            assert!(entry.eve_basis == '+' || entry.eve_basis == 'x');
        } else {
            saw_clean = true;
            assert_eq!(entry.eve_basis, '-');
        }
        assert_eq!(entry.bit_error, entry.alice_bit != entry.bob_bit);
    }
    assert!(saw_eve && saw_clean);
}

#[test]
fn sqlite_workset_list_delete_and_portable_envelope_error_paths() {
    let repo = WorksetRepository::open_in_memory().expect("in-memory SQLite");
    let mut st_a = WorksetState::default();
    st_a.active_module = ModuleId::K1Bb84Qkd;
    let mut st_b = WorksetState::default();
    st_b.active_module = ModuleId::Q2CircuitEntanglement;

    repo.save_slot("alpha", &st_a).expect("save alpha");
    repo.save_slot("beta", &st_b).expect("save beta");
    let list = repo.list_slots().expect("list slots");
    assert_eq!(list.len(), 2);
    assert!(list.iter().any(|s| s.slot_name == "alpha" && s.active_module == "K1"));
    assert!(list.iter().any(|s| s.slot_name == "beta" && s.active_module == "Q2"));

    assert!(repo.delete_slot("alpha").expect("delete alpha"));
    assert!(!repo.delete_slot("nonexistent").expect("delete nonexistent"));
    assert!(repo.load_slot("alpha").is_err());

    let list_after = repo.list_slots().expect("list after delete");
    assert_eq!(list_after.len(), 1);
    assert_eq!(list_after[0].slot_name, "beta");

    assert!(PortableWorksetEnvelope::load_from_path(Path::new("nonexistent_dir_12345/missing.qworkset.json")).is_err());
    let env = PortableWorksetEnvelope::pack("beta", &st_b).expect("pack");
    assert!(env.save_to_path(Path::new("nonexistent_dir_12345/out.qworkset.json")).is_err());
}

#[test]
fn gui_and_plotter_tofu_free_and_eve_interceptor_and_icon_invariants() {
    let plotter_src = include_str!("../src/plotter.rs");
    let gui_src = include_str!("../src/gui.rs");
    let qudit_src = include_str!("../src/qudit.rs");
    assert!(!plotter_src.contains('\u{27E9}'), "Found tofu-prone U+27E9 in src/plotter.rs");
    assert!(!gui_src.contains('\u{27E9}'), "Found tofu-prone U+27E9 in src/gui.rs");
    assert!(!qudit_src.contains('\u{27E9}'), "Found tofu-prone U+27E9 in src/qudit.rs");
    assert!(!plotter_src.contains("Intersector"), "Stale 'Intersector' typo remains in src/plotter.rs");
    assert!(plotter_src.contains("Eve Interceptor"), "Expected 'Eve Interceptor' in src/plotter.rs");
    assert!(gui_src.contains("Copyright CRV Science Network"));

    let rgba = app_icon_rgba(64);
    assert_eq!(rgba.len(), 64 * 64 * 4);
    assert!(rgba.iter().skip(3).step_by(4).any(|&a| a > 0));
}

#[test]
fn openqasm_rejects_usize_max_infer_overflow_and_non_finite_angles() {
    let overflow_qasm = "OPENQASM 2.0;\nh q[18446744073709551615];\n";
    let err_overflow = QuantumCircuit::from_openqasm(overflow_qasm)
        .expect_err("usize::MAX qubit index without qreg must return Err, not panic or wrap");
    assert!(err_overflow.contains("maximum supported is 12"), "Unexpected error: {}", err_overflow);

    let inf_angle_qasm = "OPENQASM 2.0;\nqreg q[1];\nrx(1e999) q[0];\n";
    assert!(
        QuantumCircuit::from_openqasm(inf_angle_qasm).is_err(),
        "1e999 angle literal must return Err instead of infinity/NaN"
    );

    let mul_overflow_qasm = "OPENQASM 2.0;\nqreg q[1];\nrx(1e308 * 10) q[0];\n";
    assert!(
        QuantumCircuit::from_openqasm(mul_overflow_qasm).is_err(),
        "Overflowing angle product must return Err instead of infinity/NaN"
    );
}

#[test]
fn qudit_basis_labels_use_ascii_ket_without_tofu_glyph() {
    let reg = QuditRegister::zero_state(3, 2);
    for idx in 0..reg.amps.len() {
        let lbl = reg.basis_label(idx);
        assert!(!lbl.contains('\u{27E9}'), "Found U+27E9 in basis_label: {}", lbl);
        assert!(lbl.starts_with('|') && lbl.ends_with('>'), "Expected ASCII ket |...>, got {}", lbl);
    }
    assert_eq!(reg.basis_label(0), "|00>");
    assert_eq!(reg.basis_label(1), "|01>");

    let rep = run_qudit_lab(4, QuditPreset::BellMaximallyEntangledPair, 0, 0);
    assert!(rep.basis_labels.iter().all(|l| !l.contains('\u{27E9}')));
}
