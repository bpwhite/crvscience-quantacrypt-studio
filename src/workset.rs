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

//! Bundled SQLite WAL workset save-slot repository (`quantacrypt_worksets.sqlite3`)
//! and portable SHA-256 verified `.qworkset.json` workset envelopes (`v2.2.0`, 23 modules).

use crate::curriculum::ModuleId;
use crate::density::{NoiseChannelKind, QecCodeKind};
use crate::pqc_lattice::MlKemProfile;
use crate::qudit::QuditPreset;
use crate::rng_sha256::sha256_hex;
use crate::statevector::{GateOp, QuantumCircuit};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Complete serializable workstation state across all 23 passive stochastic modules.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorksetState {
    pub active_module: ModuleId,
    pub completed_challenges: BTreeSet<ModuleId>,
    pub seed: u64,

    // Group 0: Basic Quantum Intro (I1..I4)
    pub i1_slit_sep: f64,
    pub i1_which_path: f64,
    pub i2_phase: f64,
    pub i2_arm_blocker: bool,
    pub i3_mid_angle_deg: f64,
    pub i3_insert_mid_filter: bool,
    pub i4_stages_n: usize,

    // Group 1: Foundations & Open Systems (Q1..Q4)
    pub q1_ops: Vec<GateOp>,
    pub q1_theta: f64,
    pub q1_phi: f64,
    pub q1_use_custom_angles: bool,
    pub q2_circuit: QuantumCircuit,
    pub q2_step_cursor: usize,
    pub q2_bipartite_cut: usize,
    pub q2_qasm_buffer: String,
    pub q3_theta: f64,
    pub q3_phi: f64,
    pub q3_depol_p: f64,
    pub q4_noise_kind: NoiseChannelKind,
    pub q4_step_noise: f64,
    pub q4_steps: usize,
    pub q4_theta0: f64,
    pub q4_phi0: f64,
    pub q4_qec_code: QecCodeKind,
    pub q4_physical_p: f64,
    pub q4_injected_fault: Option<usize>,
    pub q5_repeater_hops: usize,
    pub q5_link_depol: f64,
    pub q5_linear_optics_bsm: bool,

    // Group 2: High-Dimensional Qudits (D1..D2)
    pub d1_dimension_d: usize,
    pub d1_preset: QuditPreset,
    pub d1_shift_a: usize,
    pub d1_clock_b: usize,
    pub d2_dimension_d: usize,
    pub d2_mode_tilt: f64,
    pub d2_depol_p: f64,

    // Group 3: Photonic QKD (K1..K3)
    pub k1_pulses: usize,
    pub k1_eve_rate: f64,
    pub k1_channel_noise: f64,
    pub k2_pairs: usize,
    pub k2_eve_decoherence: f64,
    pub k2_visibility: f64,
    pub k3_pulses: usize,
    pub k3_angle_deg: f64,
    pub k3_eve_rate: f64,

    // Group 4: Cryptanalysis & Post-Quantum (C1..C4)
    pub c1_modulus_n: u64,
    pub c1_base_a: u64,
    pub c1_counting_qubits: usize,
    pub c2_key_bits: usize,
    pub c2_secret_key: usize,
    pub c2_iterations: usize,
    pub c3_profile: MlKemProfile,
    pub c3_eta: usize,
    pub c4_gamma1: i32,
    pub c4_eta: i32,
    pub c4_tau: usize,
    pub c4_tamper: bool,

    // Group 5: Advanced Fault-Tolerant & Variational Computing (A1..A4)
    pub a1_code_distance: usize,
    pub a1_phys_error: f64,
    pub a2_bond_r_angstrom: f64,
    pub a2_shot_noise: f64,
    pub a3_depth_p: usize,
    pub a3_drive_gamma: f64,
    pub a4_raw_t_error: f64,
    pub a5_chain_depth: usize,
    pub a5_raw_ancilla_p: f64,
    pub a5_use_distilled: bool,
}

impl Default for WorksetState {
    fn default() -> Self {
        let q2_circuit = QuantumCircuit::preset_bell_phi_plus();
        let q2_qasm_buffer = q2_circuit.to_openqasm();
        let q2_step_cursor = q2_circuit.ops.len();
        Self {
            active_module: ModuleId::I1DoubleSlit,
            completed_challenges: BTreeSet::new(),
            seed: 2026_1114,
            i1_slit_sep: 4.5,
            i1_which_path: 0.0,
            i2_phase: 0.0,
            i2_arm_blocker: true,
            i3_mid_angle_deg: 90.0,
            i3_insert_mid_filter: true,
            i4_stages_n: 8,
            q1_ops: vec![GateOp::H(0)],
            q1_theta: std::f64::consts::FRAC_PI_3,
            q1_phi: std::f64::consts::FRAC_PI_4,
            q1_use_custom_angles: false,
            q2_circuit,
            q2_step_cursor,
            q2_bipartite_cut: 1,
            q2_qasm_buffer,
            q3_theta: 1.15,
            q3_phi: 0.85,
            q3_depol_p: 0.02,
            q4_noise_kind: NoiseChannelKind::AmplitudeDampingT1,
            q4_step_noise: 0.08,
            q4_steps: 24,
            q4_theta0: std::f64::consts::FRAC_PI_2,
            q4_phi0: 0.4,
            q4_qec_code: QecCodeKind::ThreeQubitBitFlip,
            q4_physical_p: 0.08,
            q4_injected_fault: Some(1),
            q5_repeater_hops: 2,
            q5_link_depol: 0.03,
            q5_linear_optics_bsm: true,
            d1_dimension_d: 4,
            d1_preset: QuditPreset::BellMaximallyEntangledPair,
            d1_shift_a: 1,
            d1_clock_b: 0,
            d2_dimension_d: 3,
            d2_mode_tilt: 0.55,
            d2_depol_p: 0.08,
            k1_pulses: 1024,
            k1_eve_rate: 0.12,
            k1_channel_noise: 0.015,
            k2_pairs: 4000,
            k2_eve_decoherence: 0.05,
            k2_visibility: 0.98,
            k3_pulses: 2000,
            k3_angle_deg: 45.0,
            k3_eve_rate: 0.04,
            c1_modulus_n: 35,
            c1_base_a: 2,
            c1_counting_qubits: 8,
            c2_key_bits: 6,
            c2_secret_key: 42,
            c2_iterations: 6,
            c3_profile: MlKemProfile::MlKem768,
            c3_eta: 2,
            c4_gamma1: 128,
            c4_eta: 2,
            c4_tau: 8,
            c4_tamper: true,
            a1_code_distance: 5,
            a1_phys_error: 0.006,
            a2_bond_r_angstrom: 0.74,
            a2_shot_noise: 0.025,
            a3_depth_p: 3,
            a3_drive_gamma: 0.85,
            a4_raw_t_error: 0.02,
            a5_chain_depth: 6,
            a5_raw_ancilla_p: 0.04,
            a5_use_distilled: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortableWorksetEnvelope {
    pub format: String,
    pub version: String,
    pub label: String,
    pub saved_unix_secs: u64,
    pub payload_json: String,
    pub sha256_hex: String,
}

impl PortableWorksetEnvelope {
    pub fn pack(label: &str, state: &WorksetState) -> Result<Self, String> {
        let payload_json =
            serde_json::to_string_pretty(state).map_err(|e| format!("Serialize error: {}", e))?;
        let digest = sha256_hex(payload_json.as_bytes());
        let saved_unix_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Ok(Self {
            format: "quantacrypt-workset".to_string(),
            version: "2.2.0".to_string(),
            label: label.to_string(),
            saved_unix_secs,
            payload_json,
            sha256_hex: digest,
        })
    }

    pub fn unpack(&self) -> Result<WorksetState, String> {
        if self.format != "quantacrypt-workset" {
            return Err(format!("Unexpected workset format '{}'", self.format));
        }
        let actual = sha256_hex(self.payload_json.as_bytes());
        if actual != self.sha256_hex {
            return Err(format!(
                "SHA-256 workset integrity check failed (expected {}, got {})",
                self.sha256_hex, actual
            ));
        }
        serde_json::from_str(&self.payload_json).map_err(|e| format!("Invalid workset JSON: {}", e))
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), String> {
        let text =
            serde_json::to_string_pretty(self).map_err(|e| format!("Encode error: {}", e))?;
        fs::write(path, text).map_err(|e| format!("Failed writing {}: {}", path.display(), e))
    }

    pub fn load_from_path(path: &Path) -> Result<(String, WorksetState), String> {
        let raw = fs::read_to_string(path)
            .map_err(|e| format!("Failed reading {}: {}", path.display(), e))?;
        let env: Self =
            serde_json::from_str(&raw).map_err(|e| format!("Envelope parse error: {}", e))?;
        let state = env.unpack()?;
        Ok((env.label, state))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorksetSlotSummary {
    pub slot_name: String,
    pub active_module: String,
    pub mastered_count: usize,
    pub updated_unix: u64,
}

pub struct WorksetRepository {
    conn: Connection,
}

impl WorksetRepository {
    pub fn open_file(db_path: &Path) -> Result<Self, String> {
        let conn = Connection::open(db_path)
            .map_err(|e| format!("SQLite open error ({}): {}", db_path.display(), e))?;
        let repo = Self { conn };
        repo.init_schema()?;
        Ok(repo)
    }

    pub fn open_in_memory() -> Result<Self, String> {
        let conn =
            Connection::open_in_memory().map_err(|e| format!("SQLite memory error: {}", e))?;
        let repo = Self { conn };
        repo.init_schema()?;
        Ok(repo)
    }

    fn init_schema(&self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "PRAGMA journal_mode = WAL;
                 PRAGMA synchronous = NORMAL;
                 CREATE TABLE IF NOT EXISTS workset_slots (
                     slot_name TEXT PRIMARY KEY,
                     active_module TEXT NOT NULL,
                     mastered_count INTEGER NOT NULL,
                     updated_unix INTEGER NOT NULL,
                     payload_json TEXT NOT NULL,
                     sha256_hex TEXT NOT NULL
                 );",
            )
            .map_err(|e| format!("SQLite schema init error: {}", e))
    }

    pub fn save_slot(&self, slot_name: &str, state: &WorksetState) -> Result<String, String> {
        let env = PortableWorksetEnvelope::pack(slot_name, state)?;
        self.conn
            .execute(
                "INSERT INTO workset_slots (slot_name, active_module, mastered_count, updated_unix, payload_json, sha256_hex)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(slot_name) DO UPDATE SET
                     active_module = excluded.active_module,
                     mastered_count = excluded.mastered_count,
                     updated_unix = excluded.updated_unix,
                     payload_json = excluded.payload_json,
                     sha256_hex = excluded.sha256_hex;",
                params![
                    slot_name,
                    state.active_module.code(),
                    state.completed_challenges.len() as i64,
                    env.saved_unix_secs as i64,
                    env.payload_json,
                    env.sha256_hex,
                ],
            )
            .map_err(|e| format!("SQLite save_slot error: {}", e))?;
        Ok(env.sha256_hex)
    }

    pub fn load_slot(&self, slot_name: &str) -> Result<WorksetState, String> {
        let (payload_json, sha256_stored, saved_unix): (String, String, i64) = self
            .conn
            .query_row(
                "SELECT payload_json, sha256_hex, updated_unix FROM workset_slots WHERE slot_name = ?1",
                params![slot_name],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|e| format!("Slot '{}' not found ({})", slot_name, e))?;

        let env = PortableWorksetEnvelope {
            format: "quantacrypt-workset".to_string(),
            version: "2.2.0".to_string(),
            label: slot_name.to_string(),
            saved_unix_secs: saved_unix.max(0) as u64,
            payload_json,
            sha256_hex: sha256_stored,
        };
        env.unpack()
    }

    pub fn list_slots(&self) -> Result<Vec<WorksetSlotSummary>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT slot_name, active_module, mastered_count, updated_unix
                 FROM workset_slots
                 ORDER BY updated_unix DESC, slot_name ASC",
            )
            .map_err(|e| format!("SQLite list_slots prepare error: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                let mastered: i64 = row.get(2)?;
                let updated: i64 = row.get(3)?;
                Ok(WorksetSlotSummary {
                    slot_name: row.get(0)?,
                    active_module: row.get(1)?,
                    mastered_count: mastered.max(0) as usize,
                    updated_unix: updated.max(0) as u64,
                })
            })
            .map_err(|e| format!("SQLite list_slots query error: {}", e))?;
        let mut out = Vec::new();
        for item in rows {
            out.push(item.map_err(|e| format!("SQLite list_slots row error: {}", e))?);
        }
        Ok(out)
    }

    pub fn delete_slot(&self, slot_name: &str) -> Result<bool, String> {
        let affected = self
            .conn
            .execute(
                "DELETE FROM workset_slots WHERE slot_name = ?1",
                params![slot_name],
            )
            .map_err(|e| format!("SQLite delete_slot error: {}", e))?;
        Ok(affected > 0)
    }
}
