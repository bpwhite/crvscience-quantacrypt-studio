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

use clap::{Parser, Subcommand};
use eframe::egui;
use quantacrypt_studio::cryptanalysis::{run_grover_key_search, run_shor_cryptanalysis};
use quantacrypt_studio::curriculum::ModuleId;
use quantacrypt_studio::gui::{render_headless_screenshot, QuantaCryptApp};
use quantacrypt_studio::pqc_lattice::{run_ml_kem_lab, MlKemProfile};
use quantacrypt_studio::qkd::{simulate_b92, simulate_bb84, simulate_e91};
use quantacrypt_studio::qudit::{run_qudit_lab, QuditPreset};
use quantacrypt_studio::statevector::{QuantumCircuit, Statevector};
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "quantacrypt-studio",
    version = "2.2.0",
    about = "Native safe-Rust quantum simulation & cryptography learning workstation (GUI + headless CLI)"
)]
struct Cli {
    /// Launch directly into the active module workspace, skipping the 4-group Curriculum Hub
    #[arg(long)]
    skip_hub: bool,

    /// Open a specific curriculum module by code or slug (e.g. q1, q2-circuit, d1-qudit, k1-bb84, c1-shor, c3-ml-kem)
    #[arg(long)]
    module: Option<String>,

    /// Path to SQLite WAL workset save database
    #[arg(long, default_value = "quantacrypt_worksets.sqlite3")]
    db: PathBuf,

    /// Render a 1600x1000 offscreen GPU PNG screenshot to this path and exit
    #[arg(long)]
    headless_screenshot: Option<PathBuf>,

    /// View preset for `--headless-screenshot` (1 = Circuit/Statevector, 2 = Photonic QKD BB84, 3 = Lattice PQC ML-KEM)
    #[arg(long, default_value_t = 1)]
    screenshot_view: u8,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Parse and simulate an OpenQASM 2.0 file headlessly, printing probabilities and entanglement entropy
    RunQasm {
        /// Path to `.qasm` circuit file
        path: PathBuf,
        /// Number of projective Z-basis measurement shots
        #[arg(long, default_value_t = 1024)]
        shots: usize,
        /// Deterministic RNG seed
        #[arg(long, default_value_t = 2026)]
        seed: u64,
    },
    /// Run headless Quantum Key Distribution simulation (`bb84`, `e91`, or `b92`)
    SimulateQkd {
        #[arg(long, default_value = "bb84")]
        protocol: String,
        #[arg(long, default_value_t = 2048)]
        pulses: usize,
        #[arg(long, default_value_t = 0.0)]
        eve: f64,
        #[arg(long, default_value_t = 2026)]
        seed: u64,
    },
    /// Run Shor's coherent quantum period-finding and RSA semiprime factorization
    ShorFactor {
        #[arg(long, default_value_t = 35)]
        modulus: u64,
        #[arg(long, default_value_t = 2)]
        base: u64,
        #[arg(long, default_value_t = 8)]
        counting_qubits: usize,
    },
    /// Run Grover's amplitude amplification key-recovery benchmark
    GroverSearch {
        #[arg(long, default_value_t = 8)]
        key_bits: usize,
        #[arg(long, default_value_t = 173)]
        secret_key: usize,
        #[arg(long)]
        iterations: Option<usize>,
    },
    /// Run NIST FIPS 203 ML-KEM negacyclic polynomial ring encapsulation/decapsulation demo
    MlKemDemo {
        /// Profile: `ped16`, `512`, `768`, or `1024`
        #[arg(long, default_value = "768")]
        profile: String,
        #[arg(long, default_value_t = 2)]
        eta: usize,
        #[arg(long, default_value_t = 2026)]
        seed: u64,
    },
    /// Run high-dimensional qudit (`d = 2..8`) entanglement & QKD capacity benchmark
    QuditBench {
        #[arg(long, default_value_t = 4)]
        dimension: usize,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if let Some(cmd) = cli.command {
        match cmd {
            Commands::RunQasm { path, shots, seed } => {
                let src = fs::read_to_string(&path)?;
                let circuit = QuantumCircuit::from_openqasm(&src)
                    .map_err(|e| format!("OpenQASM parse failure: {}", e))?;
                let sv = Statevector::execute_circuit(&circuit, None, seed);
                let ebits = sv.bipartite_entanglement_ebits(1);
                let counts = sv.sample_counts(shots, seed ^ 0x55AA);

                println!(
                    "Circuit: {} ({} qubits, {} operations)",
                    path.display(),
                    circuit.num_qubits,
                    circuit.ops.len()
                );
                println!("Bipartite Entanglement S(rho_0): {:.6} ebits", ebits);
                println!("Top Sampled Basis Outcomes ({} shots):", shots);
                let mut pairs: Vec<(usize, usize)> = counts.into_iter().collect();
                pairs.sort_by(|a, b| b.1.cmp(&a.1));
                for (idx, cnt) in pairs.into_iter().take(12) {
                    println!(
                        "  |{:0width$b}> : {:5} ({:6.2}%)",
                        idx,
                        cnt,
                        (cnt as f64) * 100.0 / (shots as f64),
                        width = circuit.num_qubits
                    );
                }
            }
            Commands::SimulateQkd {
                protocol,
                pulses,
                eve,
                seed,
            } => match protocol.to_ascii_lowercase().as_str() {
                "bb84" => {
                    let rep = simulate_bb84(pulses, eve, 0.01, seed);
                    println!("=== BB84 Prepare-and-Measure QKD ===");
                    println!(
                        "Pulses: {} | Sifted: {} | QBER: {:.2}% | Shor-Preskill Rate: {:.4} | Secure: {}",
                        rep.total_pulses,
                        rep.sifted_len,
                        rep.observed_qber * 100.0,
                        rep.shor_preskill_rate,
                        rep.channel_secure
                    );
                    println!(
                        "Amplified Key ({} bits, identical={}): {}",
                        rep.final_amplified_key_bits, rep.keys_identical, rep.alice_final_key_hex
                    );
                }
                "e91" => {
                    let rep = simulate_e91(pulses, eve, 0.99, seed);
                    println!("=== E91 Entanglement CHSH QKD ===");
                    println!(
                        "Pairs: {} | Observed CHSH |S| = {:.4} (Bell Violated: {}) | Sifted QBER: {:.2}%",
                        rep.entangled_pairs,
                        rep.chsh_s_observed,
                        rep.bell_violated,
                        rep.sifted_qber * 100.0
                    );
                }
                "b92" => {
                    let rep = simulate_b92(pulses, 45.0, eve, seed);
                    println!("=== B92 Non-Orthogonal Two-State QKD ===");
                    println!(
                        "Pulses: {} | Conclusive Sifted: {} ({:.2}%) | Observed QBER: {:.2}%",
                        rep.total_pulses,
                        rep.sifted_conclusive_len,
                        rep.theoretical_conclusive_rate * 100.0,
                        rep.observed_qber * 100.0
                    );
                }
                other => {
                    return Err(format!("Unknown protocol '{}': expected bb84, e91, or b92", other).into());
                }
            },
            Commands::ShorFactor {
                modulus,
                base,
                counting_qubits,
            } => {
                let rep = run_shor_cryptanalysis(modulus, base, counting_qubits);
                println!("=== Shor Quantum Period-Finding & RSA Recovery ===");
                println!("{}", rep.explanation);
                println!(
                    "Quantum Period r = {} | Factors = {:?} | RSA Private d = {:?} | Plaintext {} -> Cipher {} -> Decrypted {:?}",
                    rep.quantum_period_r,
                    rep.recovered_factors,
                    rep.rsa_recovered_d,
                    rep.sample_plaintext_m,
                    rep.rsa_ciphertext_c,
                    rep.rsa_decrypted_m
                );
            }
            Commands::GroverSearch {
                key_bits,
                secret_key,
                iterations,
            } => {
                let probe = run_grover_key_search(key_bits, secret_key, 1);
                let k = iterations.unwrap_or(probe.optimal_iterations);
                let rep = run_grover_key_search(key_bits, secret_key, k);
                println!("=== Grover Symmetric Cipher Key Search ===");
                println!(
                    "Key Space N = {} | Optimal k = {} | Executed k = {} | Target Key #{} Prob = {:.2}%",
                    rep.search_space_n,
                    rep.optimal_iterations,
                    rep.selected_iterations,
                    rep.secret_key,
                    rep.target_success_prob * 100.0
                );
            }
            Commands::MlKemDemo { profile, eta, seed } => {
                let prof = match profile.as_str() {
                    "ped16" => MlKemProfile::Pedagogical16,
                    "512" => MlKemProfile::MlKem512,
                    "1024" => MlKemProfile::MlKem1024,
                    _ => MlKemProfile::MlKem768,
                };
                let rep = run_ml_kem_lab(prof, eta, seed);
                println!("=== NIST FIPS 203 ML-KEM Lattice Demo ===");
                println!(
                    "Profile: {} | Max |e_dec| = {} / {} | Bit Mismatches = {} | Exact = {}",
                    prof.label(),
                    rep.max_abs_decryption_noise,
                    rep.quarter_q_decision_bound,
                    rep.bit_mismatches,
                    rep.decapsulation_exact
                );
                println!("Shared Secret SHA-256: {}", rep.sender_shared_secret_hex);
            }
            Commands::QuditBench { dimension } => {
                let rep = run_qudit_lab(dimension, QuditPreset::BellMaximallyEntangledPair, 0, 0);
                println!("=== High-Dimensional Qudit Benchmark ===");
                println!(
                    "Dimension d = {} | Entanglement = {:.4} ebits | Weyl Residual = {:.2e} | QKD Tolerated QBER ~ {:.1}%",
                    rep.dimension_d, rep.entanglement_ebits, rep.weyl_residual, rep.tolerated_qber_pct
                );
            }
        }
        return Ok(());
    }

    if let Some(out_path) = cli.headless_screenshot {
        render_headless_screenshot(cli.db, &out_path, cli.screenshot_view)?;
        return Ok(());
    }

    let initial_mod = match &cli.module {
        Some(slug) => Some(
            ModuleId::from_slug(slug)
                .ok_or_else(|| format!("Unrecognized module code or slug '{}'", slug))?,
        ),
        None => None,
    };

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1320.0, 840.0])
            .with_min_inner_size([960.0, 640.0])
            .with_title("QuantaCrypt Studio v2.2.0 — Quantum Simulation & Cryptography Workstation"),
        ..Default::default()
    };

    let db_path = cli.db.clone();
    let skip_hub = cli.skip_hub;
    eframe::run_native(
        "QuantaCrypt Studio",
        native_options,
        Box::new(move |_cc| Ok(Box::new(QuantaCryptApp::new(db_path, skip_hub, initial_mod)))),
    )
    .map_err(|e| format!("GUI window error: {}", e).into())
}
