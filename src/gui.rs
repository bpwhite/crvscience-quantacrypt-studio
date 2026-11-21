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

//! Native `eframe` / `egui` 23-module passive real-time stochastic observatory GUI (`v2.2.0`).

use crate::cryptanalysis::{run_grover_key_search, SHOR_PRESET_SEMIPRIMES};
use crate::curriculum::{CurriculumGroup, ModuleId};
use crate::density::NoiseChannelKind;
use crate::plotter::{
    draw_amplitude_phase_bars, draw_amplitude_phase_bars_with_overlay, draw_bb84_bit_tape,
    draw_bloch_sphere_cloud, draw_circuit_diagram, draw_density_heatmap, draw_lattice_2d_cloud,
    draw_phosphor_screen, draw_photon_stream_labeled, draw_qaoa_ring_graph, draw_repeater_chain,
    draw_scientific_plot, draw_surface_code_grid, PlotHThreshold, PlotSeries,
};
use crate::pqc_lattice::{run_ml_kem_lab, MlKemProfile};
use crate::qudit::{run_qudit_lab, QuditPreset};
use crate::statevector::{GateOp, QuantumCircuit, Statevector};
use crate::stochastic_sim::{
    double_slit_analytical_bin_probs, h2_fci_and_ansatz_energy, StochasticArena,
};
use crate::workset::{
    PortableWorksetEnvelope, WorksetRepository, WorksetSlotSummary, WorksetState,
};
use eframe::egui::{self, Color32, RichText, ScrollArea, Ui, Vec2};
use std::f64::consts::PI;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppStage {
    CurriculumHub,
    ModuleWorkspace,
}

pub struct QuantaCryptApp {
    pub stage: AppStage,
    pub ws: WorksetState,
    pub arena: StochasticArena,
    pub repo: Option<WorksetRepository>,
    pub status_banner: String,
    pub bloch_yaw: f32,
    pub bloch_pitch: f32,
    pub show_workset_modal: bool,
    pub show_help_modal: bool,
    pub show_about_modal: bool,
    pub slot_name_input: String,
    pub json_path_input: String,
    pub slot_list_cache: Vec<WorksetSlotSummary>,
    pub modal_feedback: Option<String>,
    pub q2_target_qubit: usize,
    pub q2_control_qubit: usize,
    pub q2_second_control: usize,
    pub q2_angle_rad: f64,
    pub q2_show_reduced_rho: bool,
    pub q2_qasm_error: Option<String>,
}

impl QuantaCryptApp {
    pub fn new(db_path: PathBuf, start_in_workspace: bool, initial_module: Option<ModuleId>) -> Self {
        let (repo, status_banner) = match WorksetRepository::open_file(&db_path) {
            Ok(r) => (
                Some(r),
                "Passive Stochastic Stream Active (23 Modules) — Space: Play/Pause | R: Reset | F5/F9: Save/Load.".to_string(),
            ),
            Err(e) => (
                None,
                format!("SQLite unavailable ({}); running without persistent slots.", e),
            ),
        };
        let slot_list_cache = repo
            .as_ref()
            .and_then(|r| r.list_slots().ok())
            .unwrap_or_default();
        let mut ws = WorksetState::default();
        if let Some(m) = initial_module {
            ws.active_module = m;
        }
        let stage = if start_in_workspace || initial_module.is_some() {
            AppStage::ModuleWorkspace
        } else {
            AppStage::CurriculumHub
        };
        let arena = StochasticArena::new(ws.seed, ws.active_module);
        Self {
            stage,
            ws,
            arena,
            repo,
            status_banner,
            bloch_yaw: 0.62,
            bloch_pitch: 0.34,
            show_workset_modal: false,
            show_help_modal: false,
            show_about_modal: false,
            slot_name_input: "session_alpha".to_string(),
            json_path_input: "session_alpha.qworkset.json".to_string(),
            slot_list_cache,
            modal_feedback: None,
            q2_target_qubit: 0,
            q2_control_qubit: 0,
            q2_second_control: 1,
            q2_angle_rad: std::f64::consts::FRAC_PI_2,
            q2_show_reduced_rho: false,
            q2_qasm_error: None,
        }
    }

    fn refresh_slot_list(&mut self) {
        if let Some(repo) = &self.repo {
            match repo.list_slots() {
                Ok(list) => self.slot_list_cache = list,
                Err(e) => {
                    let msg = format!("Failed listing SQLite slots: {}", e);
                    self.status_banner = msg.clone();
                    self.modal_feedback = Some(msg);
                }
            }
        } else {
            self.slot_list_cache.clear();
        }
    }

    fn sync_q2_indices(&mut self) {
        let n = self.ws.q2_circuit.num_qubits.max(1);
        self.q2_target_qubit = self.q2_target_qubit.min(n - 1);
        self.q2_control_qubit = self.q2_control_qubit.min(n - 1);
        self.q2_second_control = self.q2_second_control.min(n - 1);
        if n >= 2 && self.q2_control_qubit == self.q2_target_qubit {
            self.q2_target_qubit = (self.q2_control_qubit + 1) % n;
        }
        if n >= 3 && (self.q2_second_control == self.q2_control_qubit || self.q2_second_control == self.q2_target_qubit) {
            for candidate in 0..n {
                if candidate != self.q2_control_qubit && candidate != self.q2_target_qubit {
                    self.q2_second_control = candidate;
                    break;
                }
            }
        }
        self.ws.q2_bipartite_cut = self.ws.q2_bipartite_cut.clamp(1, n.saturating_sub(1).max(1));
    }

    fn push_q2_gate(&mut self, op: GateOp) {
        self.ws.q2_circuit.ops.push(op);
        self.ws.q2_step_cursor = self.ws.q2_circuit.ops.len();
        self.ws.q2_qasm_buffer = self.ws.q2_circuit.to_openqasm();
        self.q2_qasm_error = None;
        self.arena.reset_stream(self.ws.seed, ModuleId::Q2CircuitEntanglement);
    }

    fn quick_save(&mut self) {
        match &self.repo {
            Some(repo) => match repo.save_slot("quicksave", &self.ws) {
                Ok(hash) => {
                    let msg = format!("Saved preset 'quicksave' (SHA-256: {}...)", &hash[..12]);
                    self.status_banner = msg.clone();
                    self.modal_feedback = Some(msg);
                    self.refresh_slot_list();
                }
                Err(e) => {
                    let msg = format!("Quick-save error: {}", e);
                    self.status_banner = msg.clone();
                    self.modal_feedback = Some(msg);
                }
            },
            None => {
                let msg = "Quick-save failed: SQLite repository is unavailable.".to_string();
                self.status_banner = msg.clone();
                self.modal_feedback = Some(msg);
            }
        }
    }

    fn quick_load(&mut self) {
        match &self.repo {
            Some(repo) => match repo.load_slot("quicksave") {
                Ok(loaded) => {
                    self.ws = loaded;
                    self.sync_q2_indices();
                    self.arena.reset_stream(self.ws.seed, self.ws.active_module);
                    let msg = "Loaded preset 'quicksave' and restarted stream.".to_string();
                    self.status_banner = msg.clone();
                    self.modal_feedback = Some(msg);
                }
                Err(e) => {
                    let msg = format!("Quick-load error: {}", e);
                    self.status_banner = msg.clone();
                    self.modal_feedback = Some(msg);
                }
            },
            None => {
                let msg = "Quick-load failed: SQLite repository is unavailable.".to_string();
                self.status_banner = msg.clone();
                self.modal_feedback = Some(msg);
            }
        }
    }

    fn render_top_menu(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("quantacrypt_menubar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.label(RichText::new("QuantaCrypt Studio v2.2").strong().color(Color32::from_rgb(56, 189, 248)));
                ui.separator();
                if ui.selectable_label(self.stage == AppStage::CurriculumHub, "Observatory Hub (23)").clicked() {
                    self.stage = AppStage::CurriculumHub;
                }
                if ui.selectable_label(self.stage == AppStage::ModuleWorkspace && self.ws.active_module == ModuleId::Q3Teleportation, "Q3 Teleport").clicked() {
                    self.ws.active_module = ModuleId::Q3Teleportation;
                    self.arena.reset_stream(self.ws.seed, ModuleId::Q3Teleportation);
                    self.stage = AppStage::ModuleWorkspace;
                }
                if ui.selectable_label(self.stage == AppStage::ModuleWorkspace && self.ws.active_module == ModuleId::Q5EntanglementSwapping, "Q5 Swap").clicked() {
                    self.ws.active_module = ModuleId::Q5EntanglementSwapping;
                    self.arena.reset_stream(self.ws.seed, ModuleId::Q5EntanglementSwapping);
                    self.stage = AppStage::ModuleWorkspace;
                }
                ui.menu_button("Simulations", |ui| {
                    for grp in CurriculumGroup::ALL {
                        ui.label(RichText::new(grp.title()).strong());
                        for m in ModuleId::ALL.into_iter().filter(|m| m.group() == grp) {
                            if ui.button(format!("[{}] {}", m.code(), m.title())).clicked() {
                                self.ws.active_module = m;
                                self.arena.reset_stream(self.ws.seed, m);
                                self.stage = AppStage::ModuleWorkspace;
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                    }
                });
                ui.separator();
                if ui.button(if self.arena.playing { "Pause" } else { "Play" }).clicked() {
                    self.arena.playing = !self.arena.playing;
                }
                if ui.button("Reset").clicked() {
                    self.arena.reset_stream(self.ws.seed, self.ws.active_module);
                }
                ui.add(egui::Slider::new(&mut self.arena.speed_mult, 0.25..=4.0).text("Speed x").fixed_decimals(2));
                if ui.button("Presets").clicked() {
                    self.show_workset_modal = !self.show_workset_modal;
                    if self.show_workset_modal {
                        self.refresh_slot_list();
                    }
                }
                if ui.button("Citations (F1)").clicked() {
                    self.show_help_modal = !self.show_help_modal;
                }
                if ui.button("About").clicked() {
                    self.show_about_modal = !self.show_about_modal;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.monospace(format!("t = {:.1}s | Events: {}", self.arena.sim_time, self.arena.total_events));
                });
            });
        });
    }

    fn render_hub(&mut self, ui: &mut Ui) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(6.0);
            ui.heading("QuantaCrypt Studio v2.2 — 23-Module Passive Stochastic Quantum & Cryptography Observatory");
            ui.label("From foundational double-slit/Stern-Gerlach/Zeno experiments to multi-qubit circuits, qudits, QKD, post-quantum cryptography, and fault-tolerant surface codes, VQE, QAOA, and magic-state factories.");
            let mut chosen = None;
            for grp in CurriculumGroup::ALL {
                ui.separator();
                ui.label(RichText::new(grp.title()).size(15.5).strong().color(Color32::from_rgb(167, 139, 250)));
                egui::Grid::new(format!("hub_{:?}", grp)).num_columns(2).spacing([14.0, 10.0]).striped(true).show(ui, |ui| {
                    let mods: Vec<ModuleId> = ModuleId::ALL.into_iter().filter(|m| m.group() == grp).collect();
                    for (i, m) in mods.iter().enumerate() {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                if ui.button(RichText::new(format!("Observe [{}]", m.code())).strong()).clicked() {
                                    chosen = Some(*m);
                                }
                                ui.label(RichText::new(m.title()).strong());
                            });
                            ui.label(m.summary());
                        });
                        if i % 2 == 1 { ui.end_row(); }
                    }
                    if mods.len() % 2 == 1 { ui.label(""); ui.end_row(); }
                });
            }
            if let Some(m) = chosen {
                self.ws.active_module = m;
                self.arena.reset_stream(self.ws.seed, m);
                self.stage = AppStage::ModuleWorkspace;
            }
        });
    }

    fn render_left_controls(&mut self, ui: &mut Ui) {
        let m = self.ws.active_module;
        ScrollArea::vertical().show(ui, |ui| {
            ui.label(RichText::new(format!("[{}] {}", m.code(), m.title())).strong().color(Color32::from_rgb(56, 189, 248)));
            ui.label(m.summary());
            let (cit_label, cit_url) = m.citation();
            ui.hyperlink_to(format!("↗ {}", cit_label), cit_url);
            ui.separator();
            ui.label(RichText::new("Live Stochastic Stream Parameters").strong().color(Color32::from_rgb(52, 211, 153)));

            match m {
                ModuleId::I1DoubleSlit => {
                    if ui.add(egui::Slider::new(&mut self.ws.i1_slit_sep, 2.0..=8.5).text("Slit Separation d")).changed()
                        || ui.add(egui::Slider::new(&mut self.ws.i1_which_path, 0.0..=1.0).text("Which-Path Detector")).changed()
                    {
                        self.arena.reset_stream(self.ws.seed, m);
                    }
                }
                ModuleId::I2MachZehnder => {
                    if ui.add(egui::Slider::new(&mut self.ws.i2_phase, 0.0..=(2.0 * PI)).text("Arm Phase phi")).changed()
                        || ui.checkbox(&mut self.ws.i2_arm_blocker, "Insert Lower-Arm Bomb/Blocker").changed()
                    {
                        self.arena.i2_counts = [0; 3];
                    }
                }
                ModuleId::I3SternGerlach => {
                    if ui.checkbox(&mut self.ws.i3_insert_mid_filter, "Insert Middle SG2(theta) Filter").changed()
                        || ui.add(egui::Slider::new(&mut self.ws.i3_mid_angle_deg, 0.0..=180.0).text("SG2 Angle theta (deg)")).changed()
                    {
                        self.arena.i3_counts = [0; 3];
                    }
                }
                ModuleId::I4QuantumZeno => {
                    if ui.add(egui::Slider::new(&mut self.ws.i4_stages_n, 1..=32).text("Polarizer Stages N")).changed() {
                        self.arena.i4_transmitted = 0;
                        self.arena.i4_absorbed = 0;
                    }
                }
                ModuleId::Q1BlochSingleQubit => {
                    ui.add(egui::Slider::new(&mut self.ws.q1_theta, 0.1..=PI).text("Rabi Drive Strength"));
                    ui.add(egui::Slider::new(&mut self.ws.q1_phi, -PI..=PI).text("Drive Axis Azimuth"));
                }
                ModuleId::Q2CircuitEntanglement => {
                    ui.label(RichText::new("Circuit Presets").strong());
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Bell |Phi+>").clicked() {
                            self.ws.q2_circuit = QuantumCircuit::preset_bell_phi_plus();
                            self.ws.q2_step_cursor = self.ws.q2_circuit.ops.len();
                            self.ws.q2_qasm_buffer = self.ws.q2_circuit.to_openqasm();
                            self.q2_qasm_error = None;
                            self.sync_q2_indices();
                            self.arena.reset_stream(self.ws.seed, m);
                        }
                        if ui.button("4-Qubit GHZ").clicked() {
                            self.ws.q2_circuit = QuantumCircuit::preset_ghz(4);
                            self.ws.q2_step_cursor = self.ws.q2_circuit.ops.len();
                            self.ws.q2_bipartite_cut = 2;
                            self.ws.q2_qasm_buffer = self.ws.q2_circuit.to_openqasm();
                            self.q2_qasm_error = None;
                            self.sync_q2_indices();
                            self.arena.reset_stream(self.ws.seed, m);
                        }
                        if ui.button("3-Qubit QFT").clicked() {
                            self.ws.q2_circuit = QuantumCircuit::preset_qft(3);
                            self.ws.q2_step_cursor = self.ws.q2_circuit.ops.len();
                            self.ws.q2_qasm_buffer = self.ws.q2_circuit.to_openqasm();
                            self.q2_qasm_error = None;
                            self.sync_q2_indices();
                            self.arena.reset_stream(self.ws.seed, m);
                        }
                    });
                    ui.separator();

                    let mut n_q = self.ws.q2_circuit.num_qubits;
                    ui.horizontal(|ui| {
                        ui.label(format!("Qubits: {}", n_q));
                        if ui.button("-").clicked() && n_q > 1 {
                            n_q -= 1;
                            self.ws.q2_circuit.num_qubits = n_q;
                            self.ws.q2_circuit.ops.retain(|op| op.max_qubit_index() < n_q);
                            self.ws.q2_step_cursor = self.ws.q2_step_cursor.min(self.ws.q2_circuit.ops.len());
                            self.ws.q2_qasm_buffer = self.ws.q2_circuit.to_openqasm();
                            self.q2_qasm_error = None;
                            self.sync_q2_indices();
                            self.arena.reset_stream(self.ws.seed, m);
                        }
                        if ui.button("+").clicked() && n_q < 6 {
                            n_q += 1;
                            self.ws.q2_circuit.num_qubits = n_q;
                            self.ws.q2_qasm_buffer = self.ws.q2_circuit.to_openqasm();
                            self.q2_qasm_error = None;
                            self.sync_q2_indices();
                            self.arena.reset_stream(self.ws.seed, m);
                        }
                    });

                    let max_cut = n_q.saturating_sub(1).max(1);
                    self.ws.q2_bipartite_cut = self.ws.q2_bipartite_cut.clamp(1, max_cut);
                    if n_q >= 2 {
                        ui.add(egui::Slider::new(&mut self.ws.q2_bipartite_cut, 1..=max_cut).text("Cut k"));
                    }

                    let max_ops = self.ws.q2_circuit.ops.len();
                    self.ws.q2_step_cursor = self.ws.q2_step_cursor.min(max_ops);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!("Step {}/{}", self.ws.q2_step_cursor, max_ops));
                        if ui.small_button("|<").clicked() {
                            self.ws.q2_step_cursor = 0;
                            self.arena.reset_stream(self.ws.seed, m);
                        }
                        if ui.small_button("<").clicked() {
                            self.ws.q2_step_cursor = self.ws.q2_step_cursor.saturating_sub(1);
                            self.arena.reset_stream(self.ws.seed, m);
                        }
                        if ui.small_button(">").clicked() {
                            self.ws.q2_step_cursor = (self.ws.q2_step_cursor + 1).min(max_ops);
                            self.arena.reset_stream(self.ws.seed, m);
                        }
                        if ui.small_button(">|").clicked() {
                            self.ws.q2_step_cursor = max_ops;
                            self.arena.reset_stream(self.ws.seed, m);
                        }
                        if ui.small_button("Undo").clicked() && !self.ws.q2_circuit.ops.is_empty() {
                            self.ws.q2_circuit.ops.pop();
                            self.ws.q2_step_cursor = self.ws.q2_circuit.ops.len();
                            self.ws.q2_qasm_buffer = self.ws.q2_circuit.to_openqasm();
                            self.q2_qasm_error = None;
                            self.arena.reset_stream(self.ws.seed, m);
                        }
                        if ui.small_button("Clear").clicked() {
                            self.ws.q2_circuit.ops.clear();
                            self.ws.q2_step_cursor = 0;
                            self.ws.q2_qasm_buffer = self.ws.q2_circuit.to_openqasm();
                            self.q2_qasm_error = None;
                            self.arena.reset_stream(self.ws.seed, m);
                        }
                    });

                    ui.separator();
                    ui.label(RichText::new("Interactive Gate Palette").strong());
                    self.q2_target_qubit = self.q2_target_qubit.min(n_q - 1);
                    self.q2_control_qubit = self.q2_control_qubit.min(n_q - 1);
                    self.q2_second_control = self.q2_second_control.min(n_q - 1);
                    ui.add(egui::Slider::new(&mut self.q2_target_qubit, 0..=(n_q - 1)).text("Target q[t]"));
                    if n_q >= 2 {
                        ui.add(egui::Slider::new(&mut self.q2_control_qubit, 0..=(n_q - 1)).text("Control q[c]"));
                    }
                    if n_q >= 3 {
                        ui.add(egui::Slider::new(&mut self.q2_second_control, 0..=(n_q - 1)).text("Ctrl2 q[c2]"));
                    }
                    ui.add(egui::Slider::new(&mut self.q2_angle_rad, -PI..=PI).text("Angle (rad)"));
                    ui.horizontal(|ui| {
                        if ui.small_button("pi/4").clicked() { self.q2_angle_rad = std::f64::consts::FRAC_PI_4; }
                        if ui.small_button("pi/2").clicked() { self.q2_angle_rad = std::f64::consts::FRAC_PI_2; }
                        if ui.small_button("pi").clicked() { self.q2_angle_rad = PI; }
                    });

                    let t = self.q2_target_qubit;
                    let c = self.q2_control_qubit;
                    let c2 = self.q2_second_control;
                    let th = self.q2_angle_rad;

                    ui.horizontal_wrapped(|ui| {
                        if ui.small_button("H").on_hover_text("Hadamard gate (creates equal superposition (|0>+|1>)/sqrt(2) on q[t])").clicked() { self.push_q2_gate(GateOp::H(t)); }
                        if ui.small_button("X").on_hover_text("Pauli-X (NOT) gate: pi rotation about Bloch X axis on q[t]").clicked() { self.push_q2_gate(GateOp::X(t)); }
                        if ui.small_button("Y").on_hover_text("Pauli-Y gate: pi rotation about Bloch Y axis on q[t]").clicked() { self.push_q2_gate(GateOp::Y(t)); }
                        if ui.small_button("Z").on_hover_text("Pauli-Z phase-flip gate: pi rotation about Bloch Z axis on q[t]").clicked() { self.push_q2_gate(GateOp::Z(t)); }
                        if ui.small_button("S").on_hover_text("Phase gate sqrt(Z): +pi/2 Z-axis rotation on q[t]").clicked() { self.push_q2_gate(GateOp::S(t)); }
                        if ui.small_button("Sdg").on_hover_text("Inverse Phase gate (S^dagger): -pi/2 Z-axis rotation on q[t]").clicked() { self.push_q2_gate(GateOp::Sdg(t)); }
                        if ui.small_button("T").on_hover_text("T (pi/8) non-Clifford gate: +pi/4 Z-axis rotation on q[t]").clicked() { self.push_q2_gate(GateOp::T(t)); }
                        if ui.small_button("Tdg").on_hover_text("Inverse T gate (T^dagger): -pi/4 Z-axis rotation on q[t]").clicked() { self.push_q2_gate(GateOp::Tdg(t)); }
                        if ui.small_button("Rx").on_hover_text("Parametric X rotation Rx(theta) on q[t]").clicked() { self.push_q2_gate(GateOp::Rx { target: t, theta: th }); }
                        if ui.small_button("Ry").on_hover_text("Parametric Y rotation Ry(theta) on q[t]").clicked() { self.push_q2_gate(GateOp::Ry { target: t, theta: th }); }
                        if ui.small_button("Rz").on_hover_text("Parametric Z rotation Rz(theta) on q[t]").clicked() { self.push_q2_gate(GateOp::Rz { target: t, theta: th }); }
                        if ui.small_button("P").on_hover_text("Phase shift gate P(lambda): applies e^(i*lambda) to |1> on q[t]").clicked() { self.push_q2_gate(GateOp::Phase { target: t, lambda: th }); }
                        if ui.small_button("M").on_hover_text("Projective Z-basis measurement on q[t]").clicked() { self.push_q2_gate(GateOp::Measure(t)); }
                    });

                    let two_ok = n_q >= 2 && c != t;
                    let three_ok = n_q >= 3 && c != t && c2 != t && c != c2;
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .add_enabled(two_ok, egui::Button::new("CNOT").small())
                            .on_hover_text("Controlled-NOT (CX): flips target q[t] when control q[c] = |1>")
                            .on_disabled_hover_text("Requires >= 2 qubits with distinct control q[c] != target q[t]")
                            .clicked()
                        {
                            self.push_q2_gate(GateOp::Cnot { control: c, target: t });
                        }
                        if ui
                            .add_enabled(two_ok, egui::Button::new("CZ").small())
                            .on_hover_text("Controlled-Z: applies -1 phase flip to |11> on (q[c], q[t])")
                            .on_disabled_hover_text("Requires >= 2 qubits with distinct control q[c] != target q[t]")
                            .clicked()
                        {
                            self.push_q2_gate(GateOp::Cz { control: c, target: t });
                        }
                        if ui
                            .add_enabled(two_ok, egui::Button::new("CP").small())
                            .on_hover_text("Controlled-Phase CP(lambda): shifts |11> phase by e^(i*lambda)")
                            .on_disabled_hover_text("Requires >= 2 qubits with distinct control q[c] != target q[t]")
                            .clicked()
                        {
                            self.push_q2_gate(GateOp::CPhase { control: c, target: t, lambda: th });
                        }
                        if ui
                            .add_enabled(two_ok, egui::Button::new("SWAP").small())
                            .on_hover_text("SWAP gate: exchanges quantum states of q[c] and q[t]")
                            .on_disabled_hover_text("Requires >= 2 qubits with distinct q[c] != q[t]")
                            .clicked()
                        {
                            self.push_q2_gate(GateOp::Swap(c, t));
                        }
                        if ui
                            .add_enabled(three_ok, egui::Button::new("CCX").small())
                            .on_hover_text("Toffoli (CCNOT): flips target q[t] when both q[c] = |1> and q[c2] = |1>")
                            .on_disabled_hover_text("Requires >= 3 qubits with distinct q[c], q[c2], and q[t]")
                            .clicked()
                        {
                            self.push_q2_gate(GateOp::Toffoli { c1: c, c2, target: t });
                        }
                    });
                }
                ModuleId::Q3Teleportation => {
                    ui.add(egui::Slider::new(&mut self.ws.q3_theta, 0.0..=PI).text("Payload theta"));
                    ui.add(egui::Slider::new(&mut self.ws.q3_phi, -PI..=PI).text("Payload phi"));
                    ui.add(egui::Slider::new(&mut self.ws.q3_depol_p, 0.0..=0.50).text("EPR Channel Noise p"));
                }
                ModuleId::Q4OpenSystemsQec => {
                    egui::ComboBox::from_label("Kraus Channel")
                        .selected_text(self.ws.q4_noise_kind.label())
                        .show_ui(ui, |ui| {
                            for k in NoiseChannelKind::ALL {
                                ui.selectable_value(&mut self.ws.q4_noise_kind, k, k.label());
                            }
                        });
                    ui.add(egui::Slider::new(&mut self.ws.q4_step_noise, 0.01..=0.25).text("Jump Rate"));
                    ui.add(egui::Slider::new(&mut self.ws.q4_physical_p, 0.01..=0.40).text("QEC Fault Rate p"));
                }
                ModuleId::D1QuditWeylLab => {
                    if ui.add(egui::Slider::new(&mut self.ws.d1_dimension_d, 2..=8).text("Qudit Dimension d")).changed() {
                        self.arena.d1_coincidences.clear();
                    }
                    let dmax = self.ws.d1_dimension_d.saturating_sub(1);
                    ui.add(egui::Slider::new(&mut self.ws.d1_shift_a, 0..=dmax).text("Weyl Shift X_d^a"));
                }
                ModuleId::K1Bb84Qkd => {
                    if ui.add(egui::Slider::new(&mut self.ws.k1_eve_rate, 0.0..=1.0).text("Eve Intercept Rate")).changed()
                        || ui.add(egui::Slider::new(&mut self.ws.k1_channel_noise, 0.0..=0.15).text("Fiber Flip Noise")).changed()
                    {
                        self.arena.k1_sifted_total = 0;
                        self.arena.k1_sifted_errors = 0;
                    }
                }
                ModuleId::K2E91BellChsh => {
                    if ui.add(egui::Slider::new(&mut self.ws.k2_eve_decoherence, 0.0..=1.0).text("Eve Singlet Probe")).changed()
                        || ui.add(egui::Slider::new(&mut self.ws.k2_visibility, 0.65..=1.0).text("Pair Visibility")).changed()
                    {
                        self.arena.k2_corr_sum = [[0.0; 3]; 3];
                        self.arena.k2_corr_cnt = [[0; 3]; 3];
                    }
                }
                ModuleId::K3B92NonOrthogonal => {
                    if ui.add(egui::Slider::new(&mut self.ws.k3_angle_deg, 10.0..=80.0).text("Overlap Angle theta")).changed()
                        || ui.add(egui::Slider::new(&mut self.ws.k3_eve_rate, 0.0..=1.0).text("Eve Attack Rate")).changed()
                    {
                        self.arena.k3_conclusive = 0;
                        self.arena.k3_erased = 0;
                        self.arena.k3_errors = 0;
                    }
                }
                ModuleId::C1ShorRsa => {
                    ui.horizontal_wrapped(|ui| {
                        for &sp in &SHOR_PRESET_SEMIPRIMES {
                            if ui.selectable_label(self.ws.c1_modulus_n == sp, format!("N={}", sp)).clicked() {
                                self.ws.c1_modulus_n = sp;
                                self.arena.reset_stream(self.ws.seed, m);
                            }
                        }
                    });
                    let n_max = self.ws.c1_modulus_n.saturating_sub(1).max(2);
                    if ui.add(egui::Slider::new(&mut self.ws.c1_base_a, 2..=n_max).text("Base a")).changed() {
                        self.arena.reset_stream(self.ws.seed, m);
                    }
                }
                ModuleId::C2GroverSymmetric => {
                    ui.add(egui::Slider::new(&mut self.ws.c2_key_bits, 3..=9).text("Key Bits n"));
                    let max_k = (1usize << self.ws.c2_key_bits) - 1;
                    self.ws.c2_secret_key = self.ws.c2_secret_key.min(max_k);
                    ui.add(egui::Slider::new(&mut self.ws.c2_secret_key, 0..=max_k).text("Target Key"));
                }
                ModuleId::C3MlKemLattice => {
                    egui::ComboBox::from_label("FIPS 203 Profile")
                        .selected_text(self.ws.c3_profile.label())
                        .show_ui(ui, |ui| {
                            for p in MlKemProfile::ALL {
                                ui.selectable_value(&mut self.ws.c3_profile, p, p.label());
                            }
                        });
                    ui.add(egui::Slider::new(&mut self.ws.c3_eta, 1..=20).text("Binomial Noise eta"));
                }
                ModuleId::C4PqcSignatures => {
                    if ui.add(egui::Slider::new(&mut self.ws.c4_gamma1, 32..=384).text("Mask gamma_1")).changed()
                        || ui.add(egui::Slider::new(&mut self.ws.c4_eta, 1..=6).text("Secret eta")).changed()
                        || ui.add(egui::Slider::new(&mut self.ws.c4_tau, 2..=14).text("Challenge tau")).changed()
                    {
                        self.arena.c4_raw_bins.fill(0);
                        self.arena.c4_acc_bins.fill(0);
                        self.arena.c4_trials = 0;
                        self.arena.c4_accepted = 0;
                    }
                }
                ModuleId::A1SurfaceCode => {
                    ui.horizontal(|ui| {
                        ui.label("Code Distance:");
                        for d in [3usize, 5, 7] {
                            if ui.selectable_label(self.ws.a1_code_distance == d, format!("d={}", d)).clicked() {
                                self.ws.a1_code_distance = d;
                            }
                        }
                    });
                    ui.add(egui::Slider::new(&mut self.ws.a1_phys_error, 0.001..=0.025).text("Physical Error p"));
                }
                ModuleId::A2VqeHydrogen => {
                    if ui.add(egui::Slider::new(&mut self.ws.a2_bond_r_angstrom, 0.35..=2.40).text("H-H Bond R (A)")).changed() {
                        self.arena.a2_theta = 1.10;
                    }
                    ui.add(egui::Slider::new(&mut self.ws.a2_shot_noise, 0.002..=0.12).text("Measurement Shot Noise"));
                }
                ModuleId::A3QaoaMaxCut => {
                    if ui.add(egui::Slider::new(&mut self.ws.a3_depth_p, 1..=6).text("QAOA Layers p")).changed()
                        || ui.add(egui::Slider::new(&mut self.ws.a3_drive_gamma, 0.1..=2.0).text("Phase Angle gamma")).changed()
                    {
                        self.arena.a3_cut_bins.fill(0);
                    }
                }
                ModuleId::A4MagicStateFactory => {
                    if ui.add(egui::Slider::new(&mut self.ws.a4_raw_t_error, 0.002..=0.10).text("Raw |T> Error p_in")).changed() {
                        self.arena.a4_batches = 0;
                        self.arena.a4_distilled_ok = 0;
                        self.arena.a4_distilled_err = 0;
                        self.arena.a4_aborted_blocks = 0;
                    }
                }
                ModuleId::Q5EntanglementSwapping => {
                    if ui.add(egui::Slider::new(&mut self.ws.q5_repeater_hops, 1..=4).text("Repeater BSM Hops")).changed()
                        || ui.add(egui::Slider::new(&mut self.ws.q5_link_depol, 0.0..=0.25).text("Link Depolarizing p")).changed()
                        || ui.checkbox(&mut self.ws.q5_linear_optics_bsm, "50% Linear-Optical Bell Post-Selection").changed()
                    {
                        self.arena.reset_stream(self.ws.seed, m);
                    }
                }
                ModuleId::D2QuditTeleportation => {
                    if ui.add(egui::Slider::new(&mut self.ws.d2_dimension_d, 2..=8).text("Qudit Dimension d")).changed()
                        || ui.add(egui::Slider::new(&mut self.ws.d2_mode_tilt, 0.0..=1.2).text("Payload Mode Tilt")).changed()
                        || ui.add(egui::Slider::new(&mut self.ws.d2_depol_p, 0.0..=0.90).text("Isotropic Noise p")).changed()
                    {
                        self.arena.reset_stream(self.ws.seed, m);
                    }
                }
                ModuleId::A5GateTeleportation => {
                    if ui.add(egui::Slider::new(&mut self.ws.a5_chain_depth, 1..=16).text("Chained T-Gates N")).changed()
                        || ui.add(egui::Slider::new(&mut self.ws.a5_raw_ancilla_p, 0.002..=0.15).text("Raw |T> Error p_raw")).changed()
                        || ui.checkbox(&mut self.ws.a5_use_distilled, "Use A4 15-to-1 Distilled |T> Ancillas").changed()
                    {
                        self.arena.reset_stream(self.ws.seed, m);
                    }
                }
            }
        });
    }

    fn render_center_workspace(&mut self, ui: &mut Ui) {
        let m = self.ws.active_module;
        let panel_avail_h = ui.available_height();
        ScrollArea::vertical().show(ui, |ui| {
            ui.monospace(m.key_equation());
            ui.separator();
            let avail = (panel_avail_h - 34.0_f32).clamp(380.0_f32, 1200.0_f32);
            let avail_sub = (avail - 24.0_f32).max(356.0_f32);

            match m {
                ModuleId::I1DoubleSlit => {
                    let h_top = (avail * 0.44_f32).max(195.0_f32);
                    let h_bot = (avail - h_top - 8.0_f32).max(205.0_f32);
                    draw_phosphor_screen(ui, &self.arena.i1_hits, h_top, "2D Single-Quantum Phosphor Detector Screen (Dot-by-Dot Arrival)");
                    let tot = self.arena.i1_bins.iter().sum::<u64>().max(1) as f64;
                    let probs: Vec<f64> = self.arena.i1_bins.iter().map(|&c| (c as f64) / tot).collect();
                    let theory = double_slit_analytical_bin_probs(self.ws.i1_slit_sep, self.ws.i1_which_path, probs.len());
                    let labels: Vec<String> = (0..probs.len()).map(|i| format!("{}", (i as i32) - 32)).collect();
                    draw_amplitude_phase_bars_with_overlay(
                        ui,
                        &labels,
                        &probs,
                        None,
                        Some((&theory, Color32::from_rgb(251, 191, 36), "Analytical Young")),
                        h_bot,
                        "Accumulating Screen Intensity Profile I(x) vs. Analytical Young Envelope",
                    );
                }
                ModuleId::I2MachZehnder => {
                    ui.monospace(format!(
                        "Port D1 (Bright): {}  |  Port D2 (Dark / Interaction-Free): {}  |  Blocked/Absorbed: {}",
                        self.arena.i2_counts[0], self.arena.i2_counts[1], self.arena.i2_counts[2]
                    ));
                    let h_top = (avail_sub * 0.44_f32).max(175.0_f32);
                    let h_bot = (avail_sub - h_top - 8.0_f32).max(195.0_f32);
                    draw_photon_stream_labeled(ui, &self.arena.i2_photons, 2, self.ws.i2_arm_blocker, "Arm Blocker", h_top, "Mach-Zehnder Upper & Lower Interferometer Arms");
                    let tot = self.arena.i2_counts.iter().sum::<u64>().max(1) as f64;
                    let probs = vec![
                        self.arena.i2_counts[0] as f64 / tot,
                        self.arena.i2_counts[1] as f64 / tot,
                        self.arena.i2_counts[2] as f64 / tot,
                    ];
                    let labels = vec!["D1 Bright".to_string(), "D2 Dark/IFM".to_string(), "Absorbed".to_string()];
                    draw_amplitude_phase_bars(ui, &labels, &probs, None, h_bot, "Empirical Mach-Zehnder Port Outcome Ratios");
                }
                ModuleId::I3SternGerlach => {
                    ui.monospace(format!(
                        "Final SG3(+Z): {}  |  Final SG3(-Z Revived!): {}  |  Filtered at SG1/SG2: {}",
                        self.arena.i3_counts[0], self.arena.i3_counts[1], self.arena.i3_counts[2]
                    ));
                    let h_top = (avail_sub * 0.44_f32).max(175.0_f32);
                    let h_bot = (avail_sub - h_top - 8.0_f32).max(190.0_f32);
                    draw_photon_stream_labeled(ui, &self.arena.i3_atoms, 2, self.ws.i3_insert_mid_filter, "SG2 Filter", h_top, "Sequential Stern-Gerlach Spin-1/2 Atomic Beam (SG1:Z -> SG2:theta -> SG3:Z)");
                    let pass_tot = (self.arena.i3_counts[0] + self.arena.i3_counts[1]).max(1) as f64;
                    let probs = vec![self.arena.i3_counts[0] as f64 / pass_tot, self.arena.i3_counts[1] as f64 / pass_tot];
                    let labels = vec!["|+Z> Out".to_string(), "|-Z> Revived".to_string()];
                    draw_amplitude_phase_bars(ui, &labels, &probs, None, h_bot, "SG3 Output Spin Fractions");
                }
                ModuleId::I4QuantumZeno => {
                    let tot = (self.arena.i4_transmitted + self.arena.i4_absorbed).max(1) as f64;
                    let th_surv = (PI / (2.0 * (self.ws.i4_stages_n as f64))).cos().powi(2 * (self.ws.i4_stages_n as i32)) * 100.0;
                    ui.monospace(format!(
                        "Stages N = {}  |  Empirical Survival = {:.1}%  |  Zeno Theory cos^(2N)(pi/2N) = {:.1}%",
                        self.ws.i4_stages_n,
                        (self.arena.i4_transmitted as f64) * 100.0 / tot,
                        th_surv
                    ));
                    let t0 = self.arena.i4_trace.front().map(|x| x.0).unwrap_or(0.0);
                    let t1 = (t0 + 0.1).max(self.arena.sim_time);
                    let pts: Vec<(f64, f64)> = self.arena.i4_trace.iter().copied().collect();
                    draw_scientific_plot(
                        ui,
                        "Quantum Zeno Polarization Survival (%) Across N Intermediate Projective Polarizers",
                        "Time t (s)",
                        "Survival (%)",
                        (t0, t1),
                        (0.0, 102.0),
                        &[PlotSeries { label: "Empirical Survival (%)", points: &pts, color: Color32::from_rgb(56, 189, 248) }],
                        &[PlotHThreshold { y: th_surv, label: "Analytical Zeno Limit", color: Color32::from_rgb(52, 211, 153) }],
                        avail_sub.max(240.0_f32),
                    );
                }
                ModuleId::Q1BlochSingleQubit => {
                    let mean = self.arena.q1_trace_xyz.back().map(|x| x.1).unwrap_or([0.0, 0.0, 1.0]);
                    let h_top = (avail * 0.54_f32).max(260.0_f32);
                    let h_bot = (avail - h_top - 8.0_f32).max(190.0_f32);
                    draw_bloch_sphere_cloud(ui, Vec2::new(ui.available_width().min(h_top * 1.4_f32), h_top), mean, None, &self.arena.q1_walkers, &mut self.bloch_yaw, &mut self.bloch_pitch, "Stochastic Rabi & Quantum-Jump Bloch Cloud");
                    let t_min = self.arena.q1_trace_xyz.front().map(|p| p.0).unwrap_or(0.0);
                    let t_max = (t_min + 0.1).max(self.arena.sim_time);
                    let x_pts: Vec<(f64, f64)> = self.arena.q1_trace_xyz.iter().map(|&(t, b)| (t, b[0])).collect();
                    let y_pts: Vec<(f64, f64)> = self.arena.q1_trace_xyz.iter().map(|&(t, b)| (t, b[1])).collect();
                    let z_pts: Vec<(f64, f64)> = self.arena.q1_trace_xyz.iter().map(|&(t, b)| (t, b[2])).collect();
                    draw_scientific_plot(ui, "Live Ensemble (<X>, <Y>, <Z>)", "Time t (s)", "<sigma>", (t_min, t_max), (-1.05, 1.05), &[
                        PlotSeries { label: "<X>", points: &x_pts, color: Color32::from_rgb(56, 189, 248) },
                        PlotSeries { label: "<Y>", points: &y_pts, color: Color32::from_rgb(52, 211, 153) },
                        PlotSeries { label: "<Z>", points: &z_pts, color: Color32::from_rgb(251, 191, 36) },
                    ], &[], h_bot);
                }
                ModuleId::Q2CircuitEntanglement => {
                    let n = self.ws.q2_circuit.num_qubits.max(1);
                    let cut = self.ws.q2_bipartite_cut.clamp(1, n.saturating_sub(1).max(1));
                    let sv = Statevector::execute_circuit(&self.ws.q2_circuit, Some(self.ws.q2_step_cursor), self.ws.seed);
                    let ebits = sv.bipartite_entanglement_ebits(cut);
                    let use_full = n <= 4 && !self.q2_show_reduced_rho;
                    let rho = if use_full {
                        sv.full_density_matrix().unwrap_or_else(|| sv.reduced_subsystem_rho(cut.min(4)))
                    } else {
                        sv.reduced_subsystem_rho(cut.min(4))
                    };
                    let purity = rho.purity();
                    ui.monospace(format!(
                        "Qubits: {}  |  Gates: {} (Step {}/{})  |  S(rho_A, cut={}) = {:.4} ebits  |  Purity Tr(rho^2) = {:.4}",
                        n,
                        self.ws.q2_circuit.ops.len(),
                        self.ws.q2_step_cursor.min(self.ws.q2_circuit.ops.len()),
                        self.ws.q2_circuit.ops.len(),
                        cut,
                        ebits,
                        purity
                    ));

                    let h_diag = (avail_sub * 0.27_f32).clamp(155.0_f32, 230.0_f32);
                    draw_circuit_diagram(ui, &self.ws.q2_circuit, self.ws.q2_step_cursor, &self.arena.q2_fault_sparks, h_diag);

                    let rem_h = (avail_sub - h_diag - 14.0_f32).max(280.0_f32);
                    let tot = self.arena.q2_shot_counts.iter().sum::<u64>().max(1) as f64;
                    let probs: Vec<f64> = self.arena.q2_shot_counts.iter().map(|&c| (c as f64) / tot).collect();
                    let labels: Vec<String> = (0..probs.len()).map(|i| format!("|{:0w$b}>", i, w = n)).collect();

                    ui.columns(2, |cols| {
                        draw_amplitude_phase_bars(
                            &mut cols[0],
                            &labels,
                            &probs,
                            None,
                            rem_h,
                            "Accumulating Empirical Born-Rule Shot Histogram",
                        );

                        if n <= 4 {
                            cols[1].horizontal(|ui| {
                                ui.label(RichText::new("Density Matrix View:").small());
                                if ui.selectable_label(!self.q2_show_reduced_rho, "Full |psi><psi|").clicked() {
                                    self.q2_show_reduced_rho = false;
                                }
                                if ui.selectable_label(self.q2_show_reduced_rho, format!("Reduced rho_A (q[0..{}])", cut)).clicked() {
                                    self.q2_show_reduced_rho = true;
                                }
                            });
                        }
                        let heat_sz = (rem_h * 0.44_f32).clamp(130.0_f32, 220.0_f32);
                        let heat_title = if use_full {
                            format!("Full Density Matrix |psi><psi| ({}x{})", rho.dim, rho.dim)
                        } else {
                            format!("Reduced Density Matrix rho_A ({}x{})", rho.dim, rho.dim)
                        };
                        draw_density_heatmap(&mut cols[1], &rho, heat_sz, &heat_title);

                        cols[1].add_space(4.0_f32);
                        cols[1].horizontal_wrapped(|ui| {
                            ui.label(RichText::new("OpenQASM 2.0 Editor").strong());
                            if ui.small_button("Apply OpenQASM").clicked() {
                                match QuantumCircuit::from_openqasm(&self.ws.q2_qasm_buffer) {
                                    Ok(circ) => {
                                        self.ws.q2_circuit = circ;
                                        self.ws.q2_step_cursor = self.ws.q2_circuit.ops.len();
                                        self.sync_q2_indices();
                                        self.q2_qasm_error = None;
                                        self.status_banner = format!(
                                            "Applied OpenQASM 2.0 circuit ({} qubits, {} gates).",
                                            self.ws.q2_circuit.num_qubits,
                                            self.ws.q2_circuit.ops.len()
                                        );
                                        self.arena.reset_stream(self.ws.seed, ModuleId::Q2CircuitEntanglement);
                                    }
                                    Err(e) => {
                                        self.q2_qasm_error = Some(e.clone());
                                        self.status_banner = format!("OpenQASM parse error: {}", e);
                                    }
                                }
                            }
                            if ui.small_button("Sync").clicked() {
                                self.ws.q2_qasm_buffer = self.ws.q2_circuit.to_openqasm();
                                self.q2_qasm_error = None;
                            }
                            if ui.small_button("Open .qasm").clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("OpenQASM 2.0", &["qasm", "txt"])
                                    .pick_file()
                                {
                                    match std::fs::read_to_string(&path) {
                                        Ok(src) => {
                                            self.ws.q2_qasm_buffer = src.clone();
                                            match QuantumCircuit::from_openqasm(&src) {
                                                Ok(circ) => {
                                                    self.ws.q2_circuit = circ;
                                                    self.ws.q2_step_cursor = self.ws.q2_circuit.ops.len();
                                                    self.sync_q2_indices();
                                                    self.q2_qasm_error = None;
                                                    self.status_banner = format!("Loaded OpenQASM from {}", path.display());
                                                    self.arena.reset_stream(self.ws.seed, ModuleId::Q2CircuitEntanglement);
                                                }
                                                Err(e) => {
                                                    self.q2_qasm_error = Some(e.clone());
                                                    self.status_banner = format!("OpenQASM parse error in {}: {}", path.display(), e);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            let msg = format!("Failed reading {}: {}", path.display(), e);
                                            self.q2_qasm_error = Some(msg.clone());
                                            self.status_banner = msg;
                                        }
                                    }
                                }
                            }
                            if ui.small_button("Save .qasm").clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("OpenQASM 2.0", &["qasm"])
                                    .set_file_name("circuit.qasm")
                                    .save_file()
                                {
                                    match std::fs::write(&path, &self.ws.q2_qasm_buffer) {
                                        Ok(()) => {
                                            self.status_banner = format!("Saved OpenQASM to {}", path.display());
                                        }
                                        Err(e) => {
                                            let msg = format!("Failed writing {}: {}", path.display(), e);
                                            self.q2_qasm_error = Some(msg.clone());
                                            self.status_banner = msg;
                                        }
                                    }
                                }
                            }
                        });
                        if let Some(err) = &self.q2_qasm_error {
                            cols[1].colored_label(Color32::from_rgb(251, 113, 133), err);
                        }
                        let editor_h = (rem_h - heat_sz - 96.0_f32).max(105.0_f32);
                        ScrollArea::vertical().id_salt("q2_qasm_editor_scroll").max_height(editor_h).show(&mut cols[1], |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.ws.q2_qasm_buffer)
                                    .code_editor()
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(6),
                            );
                        });
                    });
                }
                ModuleId::Q3Teleportation => {
                    let src = [self.ws.q3_theta.sin() * self.ws.q3_phi.cos(), self.ws.q3_theta.sin() * self.ws.q3_phi.sin(), self.ws.q3_theta.cos()];
                    let cloud: Vec<[f64; 3]> = self.arena.q3_bob_cloud.iter().copied().collect();
                    let mean = if cloud.is_empty() { src } else {
                        let inv = 1.0 / (cloud.len() as f64);
                        [cloud.iter().map(|b| b[0]).sum::<f64>() * inv, cloud.iter().map(|b| b[1]).sum::<f64>() * inv, cloud.iter().map(|b| b[2]).sum::<f64>() * inv]
                    };
                    let h = avail.max(320.0_f32);
                    let w = ui.available_width().min(h * 1.25_f32).max(360.0_f32);
                    draw_bloch_sphere_cloud(ui, Vec2::new(w, h), mean, Some((src, "Alice Target")), &cloud, &mut self.bloch_yaw, &mut self.bloch_pitch, "Bob Teleported Ensemble vs. Alice Source");
                }
                ModuleId::Q4OpenSystemsQec => {
                    let mean = if self.arena.q4_walkers.is_empty() {
                        [1.0, 0.0, 0.0]
                    } else {
                        let inv = 1.0 / (self.arena.q4_walkers.len() as f64);
                        [
                            self.arena.q4_walkers.iter().map(|b| b[0]).sum::<f64>() * inv,
                            self.arena.q4_walkers.iter().map(|b| b[1]).sum::<f64>() * inv,
                            self.arena.q4_walkers.iter().map(|b| b[2]).sum::<f64>() * inv,
                        ]
                    };
                    let h_top = (avail * 0.52_f32).max(250.0_f32);
                    let h_bot = (avail - h_top - 8.0_f32).max(190.0_f32);
                    draw_bloch_sphere_cloud(ui, Vec2::new(ui.available_width().min(h_top * 1.4_f32), h_top), mean, None, &self.arena.q4_walkers, &mut self.bloch_yaw, &mut self.bloch_pitch, "Monte-Carlo Open-System Walker Cloud");
                    let t0 = self.arena.q4_survival_history.front().map(|x| x.0).unwrap_or(0.0);
                    let t1 = (t0 + 0.1).max(self.arena.sim_time);
                    let u_pts: Vec<(f64, f64)> = self.arena.q4_survival_history.iter().map(|&(t, u, _)| (t, u)).collect();
                    let c_pts: Vec<(f64, f64)> = self.arena.q4_survival_history.iter().map(|&(t, _, c)| (t, c)).collect();
                    draw_scientific_plot(ui, "Live Stabilizer Survival: 3-Qubit QEC vs. Unencoded", "Time t (s)", "Survival", (t0, t1), (0.0, 1.05), &[
                        PlotSeries { label: "Unencoded", points: &u_pts, color: Color32::from_rgb(251, 113, 133) },
                        PlotSeries { label: "3-Qubit QEC", points: &c_pts, color: Color32::from_rgb(52, 211, 153) },
                    ], &[], h_bot);
                }
                ModuleId::D1QuditWeylLab => {
                    let d = self.ws.d1_dimension_d;
                    let h_top = (avail * 0.44_f32).max(165.0_f32);
                    let h_bot = (avail - h_top - 8.0_f32).max(205.0_f32);
                    draw_photon_stream_labeled(ui, &self.arena.d1_pulses, d, false, "", h_top, "High-Dimensional Optical Rail / OAM Stream");
                    let rep = run_qudit_lab(d, QuditPreset::BellMaximallyEntangledPair, self.ws.d1_shift_a, 0);
                    let tot = self.arena.d1_coincidences.iter().sum::<u64>().max(1) as f64;
                    let probs: Vec<f64> = self.arena.d1_coincidences.iter().map(|&c| (c as f64) / tot).collect();
                    draw_amplitude_phase_bars(ui, &rep.basis_labels, &probs, Some(&rep.phases_rad), h_bot, "Live Empirical 2-Qudit Coincidence Distribution");
                }
                ModuleId::K1Bb84Qkd => {
                    let h_stream = (avail * 0.28_f32).clamp(145.0_f32, 210.0_f32);
                    let h_tape = 114.0_f32;
                    let h_plot = (avail - h_stream - h_tape - 14.0_f32).max(200.0_f32);
                    draw_photon_stream_labeled(
                        ui,
                        &self.arena.qkd_photons,
                        2,
                        self.ws.k1_eve_rate > 0.01,
                        "Eve Interceptor",
                        h_stream,
                        "Live BB84 Polarized Photon Stream (+ / x)",
                    );
                    draw_bb84_bit_tape(ui, &self.arena.k1_bit_tape, h_tape);
                    let t0 = self.arena.k1_qber_trace.front().map(|x| x.0).unwrap_or(0.0);
                    let t1 = (t0 + 0.1).max(self.arena.sim_time);
                    let q_pts: Vec<(f64, f64)> = self.arena.k1_qber_trace.iter().copied().collect();
                    draw_scientific_plot(ui, "Running Empirical QBER(t) (%) vs. 11.0% Abort Bound", "Time t (s)", "QBER (%)", (t0, t1), (0.0, 32.0), &[
                        PlotSeries { label: "Running QBER (%)", points: &q_pts, color: Color32::from_rgb(251, 113, 133) },
                    ], &[PlotHThreshold { y: 11.0, label: "11.0% Abort Threshold", color: Color32::from_rgb(251, 191, 36) }], h_plot);
                }
                ModuleId::K2E91BellChsh => {
                    let t0 = self.arena.k2_chsh_trace.front().map(|x| x.0).unwrap_or(0.0);
                    let t1 = (t0 + 0.1).max(self.arena.sim_time);
                    let s_pts: Vec<(f64, f64)> = self.arena.k2_chsh_trace.iter().copied().collect();
                    draw_scientific_plot(ui, "Live Empirical CHSH |S(t)| Estimator", "Time t (s)", "|S|", (t0, t1), (1.0, 3.0), &[
                        PlotSeries { label: "CHSH |S(t)|", points: &s_pts, color: Color32::from_rgb(56, 189, 248) },
                    ], &[
                        PlotHThreshold { y: 2.8284, label: "Tsirelson 2*sqrt(2)", color: Color32::from_rgb(52, 211, 153) },
                        PlotHThreshold { y: 2.0, label: "Classical Bell Bound 2.0", color: Color32::from_rgb(251, 113, 133) },
                    ], avail.max(240.0_f32));
                }
                ModuleId::K3B92NonOrthogonal => {
                    let t0 = self.arena.k3_yield_trace.front().map(|x| x.0).unwrap_or(0.0);
                    let t1 = (t0 + 0.1).max(self.arena.sim_time);
                    let y_pts: Vec<(f64, f64)> = self.arena.k3_yield_trace.iter().copied().collect();
                    draw_scientific_plot(ui, "Live B92 Conclusive Sifting Yield (%)", "Time t (s)", "Yield (%)", (t0, t1), (0.0, 60.0), &[
                        PlotSeries { label: "Conclusive Sift (%)", points: &y_pts, color: Color32::from_rgb(56, 189, 248) },
                    ], &[], avail.max(240.0_f32));
                }
                ModuleId::C1ShorRsa => {
                    ui.monospace(format!("Shots: {}  |  Locked Period r: {:?}  |  Factors: {:?}", self.arena.total_events, self.arena.c1_locked_r, self.arena.c1_locked_factors));
                    let tot = self.arena.c1_bin_counts.iter().sum::<u64>().max(1) as f64;
                    let probs: Vec<f64> = self.arena.c1_bin_counts.iter().map(|&c| (c as f64) / tot).collect();
                    let labels: Vec<String> = (0..probs.len()).map(|i| format!("{}", i * 4)).collect();
                    draw_amplitude_phase_bars(ui, &labels, &probs, None, avail_sub.max(240.0_f32), "Empirical Inverse-QFT Resonance Peaks Emerging Shot-by-Shot");
                }
                ModuleId::C2GroverSymmetric => {
                    let rep = run_grover_key_search(self.ws.c2_key_bits, self.ws.c2_secret_key, self.arena.c2_live_iter);
                    let cap = rep.state_probabilities.len().min(64);
                    let labels: Vec<String> = (0..cap).map(|i| format!("k{}", i)).collect();
                    draw_amplitude_phase_bars(ui, &labels, &rep.state_probabilities[..cap], None, avail.max(240.0_f32), "Live Grover Wavepacket Surge & Over-Rotation Cycle");
                }
                ModuleId::C3MlKemLattice => {
                    let rep = run_ml_kem_lab(self.ws.c3_profile, self.ws.c3_eta, self.ws.seed);
                    let t0 = self.arena.c3_max_noise_history.front().map(|x| x.0).unwrap_or(0.0);
                    let t1 = (t0 + 0.1).max(self.arena.sim_time);
                    let n_pts: Vec<(f64, f64)> = self.arena.c3_max_noise_history.iter().copied().collect();
                    let h_top = (avail * 0.44_f32).max(190.0_f32);
                    let h_bot = (avail - h_top - 8.0_f32).max(220.0_f32);
                    draw_scientific_plot(ui, "Live Max Decapsulation Noise |e_dec| vs. q/4 = 832", "Time t (s)", "Max |e_dec|", (t0, t1), (0.0, 1100.0), &[
                        PlotSeries { label: "Max |e_dec|", points: &n_pts, color: Color32::from_rgb(56, 189, 248) },
                    ], &[PlotHThreshold { y: 832.0, label: "q/4 = 832 Failure Bound", color: Color32::from_rgb(251, 113, 133) }], h_top);
                    let cloud: Vec<[f64; 2]> = self.arena.c3_lwe_cloud.iter().copied().collect();
                    draw_lattice_2d_cloud(ui, &rep.lattice_2d_points, rep.lattice_basis_b1, rep.lattice_basis_b2, rep.lattice_2d_exact_target, &cloud, Vec2::new(ui.available_width().max(340.0_f32), h_bot));
                }
                ModuleId::C4PqcSignatures => {
                    let max_c = self.arena.c4_raw_bins.iter().copied().max().unwrap_or(10).max(10) as f64 * 1.15;
                    let raw_pts: Vec<(f64, f64)> = self.arena.c4_raw_bins.iter().enumerate().map(|(i, &c)| (i as f64, c as f64)).collect();
                    let acc_pts: Vec<(f64, f64)> = self.arena.c4_acc_bins.iter().enumerate().map(|(i, &c)| (i as f64, c as f64)).collect();
                    draw_scientific_plot(ui, "Accumulating FIPS 204 z = y + c*s: Raw Tails vs. Accepted Box", "Bin Index", "Count", (0.0, 20.0), (0.0, max_c), &[
                        PlotSeries { label: "Raw z = y + c*s", points: &raw_pts, color: Color32::from_rgb(251, 113, 133) },
                        PlotSeries { label: "Accepted Box", points: &acc_pts, color: Color32::from_rgb(52, 211, 153) },
                    ], &[], avail.max(240.0_f32));
                }
                ModuleId::A1SurfaceCode => {
                    let d = self.ws.a1_code_distance;
                    let grid_sz = (avail * 0.44_f32).clamp(210.0_f32, 340.0_f32);
                    let h_bot = (avail - grid_sz - 38.0_f32).max(195.0_f32);
                    ui.horizontal(|ui| {
                        draw_surface_code_grid(ui, d, &self.arena.a1_data_errors, grid_sz);
                        ui.vertical(|ui| {
                            ui.monospace(format!("Physical Error p = {:.2}% (Threshold ~ 1.00%)", self.ws.a1_phys_error * 100.0));
                            ui.monospace(format!("d=3 Survival: {:.1}%", self.arena.a1_surv_d3 * 100.0));
                            ui.monospace(format!("d=5 Survival: {:.1}%", self.arena.a1_surv_d5 * 100.0));
                            ui.monospace(format!("d=7 Survival: {:.1}%", self.arena.a1_surv_d7 * 100.0));
                        });
                    });
                    let t0 = self.arena.a1_trace.front().map(|x| x.0).unwrap_or(0.0);
                    let t1 = (t0 + 0.1).max(self.arena.sim_time);
                    let s3: Vec<(f64, f64)> = self.arena.a1_trace.iter().map(|&(t, a, _, _)| (t, a)).collect();
                    let s5: Vec<(f64, f64)> = self.arena.a1_trace.iter().map(|&(t, _, b, _)| (t, b)).collect();
                    let s7: Vec<(f64, f64)> = self.arena.a1_trace.iter().map(|&(t, _, _, c)| (t, c)).collect();
                    draw_scientific_plot(ui, "Surface-Code Distance Scaling (Below p_th ~ 1%, d=7 Beats d=3; Above 1%, Crossover Reverses!)", "Time t (s)", "Logical Survival", (t0, t1), (0.0, 1.05), &[
                        PlotSeries { label: "d = 3", points: &s3, color: Color32::from_rgb(251, 191, 36) },
                        PlotSeries { label: "d = 5", points: &s5, color: Color32::from_rgb(56, 189, 248) },
                        PlotSeries { label: "d = 7", points: &s7, color: Color32::from_rgb(52, 211, 153) },
                    ], &[], h_bot);
                }
                ModuleId::A2VqeHydrogen => {
                    let (fci, e_cur) = h2_fci_and_ansatz_energy(self.ws.a2_bond_r_angstrom, self.arena.a2_theta);
                    ui.monospace(format!(
                        "R = {:.2} A  |  Ansatz theta = {:+.3} rad  |  VQE E = {:.4} Ha  |  Exact FCI = {:.4} Ha (Delta = {:.2} mHa)",
                        self.ws.a2_bond_r_angstrom, self.arena.a2_theta, e_cur, fci, (e_cur - fci) * 1000.0
                    ));
                    let t0 = self.arena.a2_energy_trace.front().map(|x| x.0).unwrap_or(0.0);
                    let t1 = (t0 + 0.1).max(self.arena.sim_time);
                    let v_pts: Vec<(f64, f64)> = self.arena.a2_energy_trace.iter().map(|&(t, ev, _)| (t, ev)).collect();
                    draw_scientific_plot(ui, "Live Shot-Noise SPSA VQE Descent Toward Molecular H2 FCI Ground State (Ha)", "Time t (s)", "Energy (Ha)", (t0, t1), (-1.20, -0.20), &[
                        PlotSeries { label: "VQE Ansatz E(t)", points: &v_pts, color: Color32::from_rgb(56, 189, 248) },
                    ], &[PlotHThreshold { y: fci, label: "Exact STO-3G FCI Ground Energy", color: Color32::from_rgb(52, 211, 153) }], avail_sub.max(240.0_f32));
                }
                ModuleId::A3QaoaMaxCut => {
                    let top_sz = (avail * 0.44_f32).clamp(210.0_f32, 330.0_f32);
                    let h_bot = (avail - top_sz - 38.0_f32).max(195.0_f32);
                    ui.horizontal(|ui| {
                        draw_qaoa_ring_graph(ui, self.arena.a3_last_mask, top_sz);
                        let tot = self.arena.a3_cut_bins.iter().sum::<u64>().max(1) as f64;
                        let probs: Vec<f64> = self.arena.a3_cut_bins.iter().map(|&c| (c as f64) / tot).collect();
                        let labels: Vec<String> = (0..10).map(|c| format!("C={}", c)).collect();
                        draw_amplitude_phase_bars(ui, &labels, &probs, None, top_sz + 28.0_f32, "Sampled Cut-Value Distribution (Max-Cut = 10)");
                    });
                    let t0 = self.arena.a3_ratio_trace.front().map(|x| x.0).unwrap_or(0.0);
                    let t1 = (t0 + 0.1).max(self.arena.sim_time);
                    let r_pts: Vec<(f64, f64)> = self.arena.a3_ratio_trace.iter().copied().collect();
                    draw_scientific_plot(ui, "Running QAOA Approximation Ratio <C> / C_max", "Time t (s)", "Ratio", (t0, t1), (0.4, 1.02), &[
                        PlotSeries { label: "<C> / C_max", points: &r_pts, color: Color32::from_rgb(52, 211, 153) },
                    ], &[PlotHThreshold { y: 0.6924, label: "Goemans-Williamson / p=1 Reference 0.692", color: Color32::from_rgb(251, 191, 36) }], h_bot);
                }
                ModuleId::A4MagicStateFactory => {
                    let p_in = self.ws.a4_raw_t_error;
                    let th_out = 35.0 * p_in.powi(3) * 100.0;
                    ui.monospace(format!(
                        "15-Qubit Batches: {}  |  Purified |T>: {}  |  Syndrome Aborts: {}  |  Cubic 35*p_in^3 = {:.4}%",
                        self.arena.a4_batches, self.arena.a4_distilled_ok, self.arena.a4_aborted_blocks, th_out
                    ));
                    let t0 = self.arena.a4_err_trace.front().map(|x| x.0).unwrap_or(0.0);
                    let t1 = (t0 + 0.1).max(self.arena.sim_time);
                    let e_pts: Vec<(f64, f64)> = self.arena.a4_err_trace.iter().copied().collect();
                    draw_scientific_plot(ui, "Distilled |T> Output Infidelity (%) vs. Raw Input Error p_in (%)", "Time t (s)", "Error (%)", (t0, t1), (0.0, (p_in * 120.0).max(2.0)), &[
                        PlotSeries { label: "Distilled p_out (%)", points: &e_pts, color: Color32::from_rgb(52, 211, 153) },
                    ], &[PlotHThreshold { y: p_in * 100.0, label: "Raw Unpurified Input p_in (%)", color: Color32::from_rgb(251, 113, 133) }], avail_sub.max(240.0_f32));
                }
                ModuleId::Q5EntanglementSwapping => {
                    let rate = if self.arena.q5_trials > 0 {
                        (self.arena.q5_heralded as f64) * 100.0 / (self.arena.q5_trials as f64)
                    } else {
                        0.0
                    };
                    ui.monospace(format!(
                        "Repeater Hops: {}  |  Trials: {}  |  Heralded End-to-End Pairs: {} ({:.1}%)",
                        self.ws.q5_repeater_hops, self.arena.q5_trials, self.arena.q5_heralded, rate
                    ));
                    let h_top = (avail_sub * 0.44_f32).max(175.0_f32);
                    let h_bot = (avail_sub - h_top - 8.0_f32).max(205.0_f32);
                    draw_repeater_chain(ui, self.ws.q5_repeater_hops, &self.arena.q5_swap_flags, (self.arena.sim_time * 1.4) as f32, h_top);
                    let t0 = self.arena.q5_fid_trace.front().map(|x| x.0).unwrap_or(0.0);
                    let t1 = (t0 + 0.1).max(self.arena.sim_time);
                    let f_pts: Vec<(f64, f64)> = self.arena.q5_fid_trace.iter().copied().collect();
                    draw_scientific_plot(ui, "Heralded End-to-End Bell Fidelity F(t) vs. Entanglement Bound (0.50)", "Time t (s)", "Fidelity", (t0, t1), (0.20, 1.05), &[
                        PlotSeries { label: "Swapped Bell Fidelity", points: &f_pts, color: Color32::from_rgb(52, 211, 153) },
                    ], &[PlotHThreshold { y: 0.50, label: "Separability Witness 0.50", color: Color32::from_rgb(251, 113, 133) }], h_bot);
                }
                ModuleId::D2QuditTeleportation => {
                    let d = self.ws.d2_dimension_d.clamp(2, 8);
                    let cls_lim = 2.0 / ((d + 1) as f64);
                    let sub_lim = ((d - 1) as f64) / (d as f64);
                    let emp_f = if self.arena.d2_shots > 0 {
                        self.arena.d2_fid_sum / (self.arena.d2_shots as f64)
                    } else {
                        1.0
                    };
                    ui.monospace(format!(
                        "d = {} ({}^2 = {} Bell states)  |  Last (m,n) = {:?}  |  F_avg = {:.3} (Classical = {:.3}, (d-1)/d = {:.3})",
                        d, d, d * d, self.arena.d2_last_mn, emp_f, cls_lim, sub_lim
                    ));
                    let h_top = (avail_sub * 0.48_f32).max(195.0_f32);
                    let h_bot = (avail_sub - h_top - 8.0_f32).max(185.0_f32);
                    let tot = self.arena.d2_bob_counts.iter().sum::<u64>().max(1) as f64;
                    let probs: Vec<f64> = self.arena.d2_bob_counts.iter().map(|&c| (c as f64) / tot).collect();
                    let labels: Vec<String> = (0..d).map(|k| format!("|{}>", k)).collect();
                    draw_amplitude_phase_bars(ui, &labels, &probs, None, h_top, "Bob Reconstructed d-Mode Intensity Profile After Weyl Z_d^m X_d^n");
                    let t0 = self.arena.d2_fid_trace.front().map(|x| x.0).unwrap_or(0.0);
                    let t1 = (t0 + 0.1).max(self.arena.sim_time);
                    let f_pts: Vec<(f64, f64)> = self.arena.d2_fid_trace.iter().copied().collect();
                    draw_scientific_plot(ui, "Average d-Dimensional Teleportation Fidelity vs. Dimensionality Witnesses", "Time t (s)", "F_avg", (t0, t1), (0.0, 1.05), &[
                        PlotSeries { label: "Empirical F_avg(t)", points: &f_pts, color: Color32::from_rgb(56, 189, 248) },
                    ], &[
                        PlotHThreshold { y: sub_lim, label: "(d-1)-Dim Subspace Bound", color: Color32::from_rgb(251, 191, 36) },
                        PlotHThreshold { y: cls_lim, label: "Classical 2/(d+1) Bound", color: Color32::from_rgb(251, 113, 133) },
                    ], h_bot);
                }
                ModuleId::A5GateTeleportation => {
                    let target_phi = (self.ws.a5_chain_depth as f64) * (PI * 0.25);
                    let target_vec = [target_phi.cos(), target_phi.sin(), 0.0];
                    let cloud: Vec<[f64; 3]> = self.arena.a5_cloud.iter().copied().collect();
                    let mean = if cloud.is_empty() {
                        target_vec
                    } else {
                        let inv = 1.0 / (cloud.len() as f64);
                        [
                            cloud.iter().map(|b| b[0]).sum::<f64>() * inv,
                            cloud.iter().map(|b| b[1]).sum::<f64>() * inv,
                            cloud.iter().map(|b| b[2]).sum::<f64>() * inv,
                        ]
                    };
                    let fix_pct = if self.arena.a5_total_gadgets > 0 {
                        (self.arena.a5_s_fixups as f64) * 100.0 / (self.arena.a5_total_gadgets as f64)
                    } else {
                        50.0
                    };
                    ui.monospace(format!(
                        "Chained T^{} Gadgets: {}  |  Clifford S^1 Fixups: {:.1}%  |  Ancilla Mode: {}",
                        self.ws.a5_chain_depth,
                        self.arena.a5_total_gadgets,
                        fix_pct,
                        if self.ws.a5_use_distilled { "A4 15-to-1 Distilled |T>" } else { "Raw Unpurified |T>" }
                    ));
                    let h_top = (avail_sub * 0.54_f32).max(255.0_f32);
                    let h_bot = (avail_sub - h_top - 8.0_f32).max(180.0_f32);
                    draw_bloch_sphere_cloud(ui, Vec2::new(ui.available_width().min(h_top * 1.4_f32), h_top), mean, Some((target_vec, "Ideal T^N|+>")), &cloud, &mut self.bloch_yaw, &mut self.bloch_pitch, "Teleported T^N Bloch Equator Cloud vs. Ideal Octant");
                    let t0 = self.arena.a5_fid_trace.front().map(|x| x.0).unwrap_or(0.0);
                    let t1 = (t0 + 0.1).max(self.arena.sim_time);
                    let act_pts: Vec<(f64, f64)> = self.arena.a5_fid_trace.iter().map(|&(t, f, _)| (t, f)).collect();
                    let raw_ref = self.arena.a5_fid_trace.back().map(|x| x.2).unwrap_or(0.8);
                    draw_scientific_plot(ui, "Chained Gate-Teleportation Sequence Fidelity F(T^N)", "Time t (s)", "Fidelity", (t0, t1), (0.35, 1.05), &[
                        PlotSeries { label: "Teleported T^N Fidelity", points: &act_pts, color: Color32::from_rgb(52, 211, 153) },
                    ], &[PlotHThreshold { y: raw_ref, label: "Unpurified Raw |T> Baseline", color: Color32::from_rgb(251, 113, 133) }], h_bot);
                }
            }
        });
    }

    fn render_modals(&mut self, ctx: &egui::Context) {
        if self.show_workset_modal {
            let mut open = self.show_workset_modal;
            egui::Window::new("Preset Slots (SQLite WAL) & Portable JSON")
                .open(&mut open)
                .resizable(true)
                .default_width(540.0_f32)
                .show(ctx, |ui| {
                    ui.label(RichText::new("SQLite WAL Workset Slots").strong());
                    if self.repo.is_none() {
                        ui.colored_label(
                            Color32::from_rgb(251, 191, 36),
                            "SQLite repository unavailable; only portable JSON export/import is active.",
                        );
                    }
                    ui.horizontal(|ui| {
                        ui.label("Slot:");
                        ui.text_edit_singleline(&mut self.slot_name_input);
                        if ui.button("Save Slot").clicked() {
                            let name = self.slot_name_input.trim().to_string();
                            if name.is_empty() {
                                let msg = "Slot name cannot be empty.".to_string();
                                self.status_banner = msg.clone();
                                self.modal_feedback = Some(msg);
                            } else if let Some(repo) = &self.repo {
                                match repo.save_slot(&name, &self.ws) {
                                    Ok(hash) => {
                                        let msg = format!("Saved SQLite slot '{}' (SHA-256: {}...)", name, &hash[..12]);
                                        self.status_banner = msg.clone();
                                        self.modal_feedback = Some(msg);
                                        self.refresh_slot_list();
                                    }
                                    Err(e) => {
                                        let msg = format!("SQLite save failed: {}", e);
                                        self.status_banner = msg.clone();
                                        self.modal_feedback = Some(msg);
                                    }
                                }
                            } else {
                                let msg = "SQLite repository is not open.".to_string();
                                self.status_banner = msg.clone();
                                self.modal_feedback = Some(msg);
                            }
                        }
                        if ui.button("Refresh").clicked() {
                            self.refresh_slot_list();
                        }
                    });

                    ui.add_space(4.0_f32);
                    let slots = self.slot_list_cache.clone();
                    let mut action_load: Option<String> = None;
                    let mut action_delete: Option<String> = None;
                    if slots.is_empty() {
                        ui.label(RichText::new("No saved SQLite slots yet.").weak());
                    } else {
                        ScrollArea::vertical().id_salt("sqlite_slots_scroll").max_height(160.0_f32).show(ui, |ui| {
                            egui::Grid::new("sqlite_slots_grid").striped(true).num_columns(5).show(ui, |ui| {
                                ui.label(RichText::new("Slot").strong());
                                ui.label(RichText::new("Module").strong());
                                ui.label(RichText::new("Mastered").strong());
                                ui.label("");
                                ui.label("");
                                ui.end_row();
                                for s in &slots {
                                    ui.monospace(&s.slot_name);
                                    ui.monospace(&s.active_module);
                                    ui.monospace(format!("{}", s.mastered_count));
                                    if ui.small_button("Load").clicked() {
                                        action_load = Some(s.slot_name.clone());
                                    }
                                    if ui.small_button("Delete").clicked() {
                                        action_delete = Some(s.slot_name.clone());
                                    }
                                    ui.end_row();
                                }
                            });
                        });
                    }
                    if let Some(name) = action_load {
                        if let Some(repo) = &self.repo {
                            match repo.load_slot(&name) {
                                Ok(loaded) => {
                                    self.ws = loaded;
                                    self.sync_q2_indices();
                                    self.arena.reset_stream(self.ws.seed, self.ws.active_module);
                                    let msg = format!("Loaded SQLite slot '{}'.", name);
                                    self.status_banner = msg.clone();
                                    self.modal_feedback = Some(msg);
                                }
                                Err(e) => {
                                    let msg = format!("Failed loading slot '{}': {}", name, e);
                                    self.status_banner = msg.clone();
                                    self.modal_feedback = Some(msg);
                                }
                            }
                        }
                    }
                    if let Some(name) = action_delete {
                        if let Some(repo) = &self.repo {
                            match repo.delete_slot(&name) {
                                Ok(true) => {
                                    let msg = format!("Deleted SQLite slot '{}'.", name);
                                    self.status_banner = msg.clone();
                                    self.modal_feedback = Some(msg);
                                    self.refresh_slot_list();
                                }
                                Ok(false) => {
                                    let msg = format!("Slot '{}' was not found.", name);
                                    self.status_banner = msg.clone();
                                    self.modal_feedback = Some(msg);
                                }
                                Err(e) => {
                                    let msg = format!("Failed deleting slot '{}': {}", name, e);
                                    self.status_banner = msg.clone();
                                    self.modal_feedback = Some(msg);
                                }
                            }
                        }
                    }

                    ui.separator();
                    ui.label(RichText::new("Portable Workset JSON (.qworkset.json)").strong());
                    ui.horizontal(|ui| {
                        if ui.button("Export JSON...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("QuantaCrypt Workset", &["qworkset.json", "json"])
                                .set_file_name("session_alpha.qworkset.json")
                                .save_file()
                            {
                                self.json_path_input = path.display().to_string();
                                match PortableWorksetEnvelope::pack("preset", &self.ws).and_then(|e| e.save_to_path(&path)) {
                                    Ok(()) => {
                                        let msg = format!("Exported workset to {}", path.display());
                                        self.status_banner = msg.clone();
                                        self.modal_feedback = Some(msg);
                                    }
                                    Err(e) => {
                                        let msg = format!("JSON export error: {}", e);
                                        self.status_banner = msg.clone();
                                        self.modal_feedback = Some(msg);
                                    }
                                }
                            }
                        }
                        if ui.button("Import JSON...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("QuantaCrypt Workset", &["qworkset.json", "json"])
                                .pick_file()
                            {
                                self.json_path_input = path.display().to_string();
                                match PortableWorksetEnvelope::load_from_path(&path) {
                                    Ok((lbl, st)) => {
                                        self.ws = st;
                                        self.sync_q2_indices();
                                        self.arena.reset_stream(self.ws.seed, self.ws.active_module);
                                        let msg = format!("Imported workset '{}' from {}", lbl, path.display());
                                        self.status_banner = msg.clone();
                                        self.modal_feedback = Some(msg);
                                    }
                                    Err(e) => {
                                        let msg = format!("JSON import error: {}", e);
                                        self.status_banner = msg.clone();
                                        self.modal_feedback = Some(msg);
                                    }
                                }
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.json_path_input);
                        if ui.button("Save Path").clicked() {
                            let p = PathBuf::from(self.json_path_input.trim());
                            match PortableWorksetEnvelope::pack("preset", &self.ws).and_then(|e| e.save_to_path(&p)) {
                                Ok(()) => {
                                    let msg = format!("Saved JSON workset to {}", p.display());
                                    self.status_banner = msg.clone();
                                    self.modal_feedback = Some(msg);
                                }
                                Err(e) => {
                                    let msg = format!("JSON save error: {}", e);
                                    self.status_banner = msg.clone();
                                    self.modal_feedback = Some(msg);
                                }
                            }
                        }
                        if ui.button("Load Path").clicked() {
                            let p = PathBuf::from(self.json_path_input.trim());
                            match PortableWorksetEnvelope::load_from_path(&p) {
                                Ok((lbl, st)) => {
                                    self.ws = st;
                                    self.sync_q2_indices();
                                    self.arena.reset_stream(self.ws.seed, self.ws.active_module);
                                    let msg = format!("Loaded JSON workset '{}' from {}", lbl, p.display());
                                    self.status_banner = msg.clone();
                                    self.modal_feedback = Some(msg);
                                }
                                Err(e) => {
                                    let msg = format!("JSON load error: {}", e);
                                    self.status_banner = msg.clone();
                                    self.modal_feedback = Some(msg);
                                }
                            }
                        }
                    });
                    if let Some(fb) = &self.modal_feedback {
                        ui.separator();
                        ui.monospace(fb);
                    }
                });
            self.show_workset_modal = open;
        }
        if self.show_help_modal {
            let mut open = self.show_help_modal;
            egui::Window::new("Shortcuts & Primary Literature Citations (23 Modules)").open(&mut open).resizable(true).default_width(600.0).show(ctx, |ui| {
                ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                    ui.monospace("Space : Play / Pause  |  R : Reset Stream  |  F5/F9 : Save/Load  |  Esc : Hub");
                    ui.separator();
                    for m in ModuleId::ALL {
                        let (lbl, url) = m.citation();
                        ui.hyperlink_to(format!("[{}] {}", m.code(), lbl), url);
                    }
                });
            });
            self.show_help_modal = open;
        }
        if self.show_about_modal {
            let mut open = self.show_about_modal;
            egui::Window::new("About QuantaCrypt Studio")
                .open(&mut open)
                .resizable(false)
                .collapsible(false)
                .default_width(440.0_f32)
                .show(ctx, |ui| {
                    ui.heading(format!("QuantaCrypt Studio v{}", env!("CARGO_PKG_VERSION")));
                    ui.label("Native safe-Rust 23-module passive stochastic quantum simulation, OpenQASM 2.0 circuit & density-matrix explorer, QKD, and post-quantum cryptography workstation.");
                    ui.separator();
                    ui.label(RichText::new("Copyright CRV Science Network").strong());
                    ui.label("Licensed under the GNU General Public License v3.0 or later (GPL-3.0-or-later).");
                    ui.separator();
                    ui.hyperlink_to("CRV Science Network (https://crvscience.com)", "https://crvscience.com");
                    ui.hyperlink_to("CRV Science Apps Hub (https://crvscience.com/apps)", "https://crvscience.com/apps");
                });
            self.show_about_modal = open;
        }
    }

    pub fn ui_step(&mut self, ctx: &egui::Context) {
        ctx.set_visuals(egui::Visuals::dark());
        let wants_kb = ctx.wants_keyboard_input();
        let (esc_pressed, space_pressed, r_pressed, f1_pressed, f5_pressed, f9_pressed, dt) =
            ctx.input(|i| {
                (
                    i.key_pressed(egui::Key::Escape),
                    i.key_pressed(egui::Key::Space),
                    i.key_pressed(egui::Key::R),
                    i.key_pressed(egui::Key::F1),
                    i.key_pressed(egui::Key::F5),
                    i.key_pressed(egui::Key::F9),
                    i.unstable_dt as f64,
                )
            });

        if esc_pressed {
            if self.show_about_modal {
                self.show_about_modal = false;
            } else if self.show_help_modal {
                self.show_help_modal = false;
            } else if self.show_workset_modal {
                self.show_workset_modal = false;
            } else if !wants_kb {
                self.stage = AppStage::CurriculumHub;
            }
        }
        if !wants_kb {
            if space_pressed {
                self.arena.playing = !self.arena.playing;
            }
            if r_pressed {
                self.arena.reset_stream(self.ws.seed, self.ws.active_module);
            }
        }
        if f1_pressed {
            self.show_help_modal = !self.show_help_modal;
        }
        if f5_pressed {
            self.quick_save();
        }
        if f9_pressed {
            self.quick_load();
        }
        if self.stage == AppStage::ModuleWorkspace {
            self.arena.step(dt, &self.ws);
            if self.arena.playing { ctx.request_repaint(); }
        }
        self.render_top_menu(ctx);
        egui::TopBottomPanel::bottom("quantacrypt_statusbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.monospace(&self.status_banner);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("About").clicked() {
                        self.show_about_modal = true;
                    }
                    ui.separator();
                    ui.label(RichText::new("Copyright CRV Science Network").small().color(Color32::from_rgb(148, 164, 186)));
                });
            });
        });
        match self.stage {
            AppStage::CurriculumHub => { egui::CentralPanel::default().show(ctx, |ui| { self.render_hub(ui); }); }
            AppStage::ModuleWorkspace => {
                egui::SidePanel::left("quantacrypt_left_panel").resizable(true).default_width(310.0).show(ctx, |ui| { self.render_left_controls(ui); });
                egui::CentralPanel::default().show(ctx, |ui| { self.render_center_workspace(ui); });
            }
        }
        self.render_modals(ctx);
    }
}

impl eframe::App for QuantaCryptApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui_step(ctx);
    }
}

fn block_on_future<F: std::future::Future>(fut: F) -> F::Output {
    struct NoopWake;
    impl std::task::Wake for NoopWake {
        fn wake(self: std::sync::Arc<Self>) {}
    }
    let waker = std::task::Waker::from(std::sync::Arc::new(NoopWake));
    let mut cx = std::task::Context::from_waker(&waker);
    let mut fut = std::pin::pin!(fut);
    loop {
        match fut.as_mut().poll(&mut cx) {
            std::task::Poll::Ready(val) => return val,
            std::task::Poll::Pending => std::thread::yield_now(),
        }
    }
}

fn write_rgba_png(
    path: &std::path::Path,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let expected = (width as usize) * (height as usize) * 4;
    if rgba.len() != expected {
        return Err(format!("Unexpected RGBA buffer length {} (expected {})", rgba.len(), expected).into());
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let file = std::fs::File::create(path)?;
    let w = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Balanced);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(rgba)?;
    Ok(())
}

/// Render an offscreen 1600x1000 RGBA PNG screenshot without opening any OS GUI window.
pub fn render_headless_screenshot(
    db_path: PathBuf,
    out_path: &std::path::Path,
    view: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    use eframe::egui_wgpu::{wgpu, Renderer, ScreenDescriptor};

    let width: u32 = 1600;
    let height: u32 = 1000;
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = block_on_future(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .ok_or("No GPU adapter available for offscreen rendering")?;
    let (device, queue) = block_on_future(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("quantacrypt-offscreen-device"),
            ..Default::default()
        },
        None,
    ))
    .map_err(|e| format!("Failed to request offscreen GPU device: {e}"))?;

    let format = wgpu::TextureFormat::Rgba8Unorm;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("quantacrypt-offscreen-target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bytes_per_row = width * 4;
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("quantacrypt-offscreen-staging"),
        size: (bytes_per_row * height) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut renderer = Renderer::new(&device, format, None, 1, false);
    let egui_ctx = egui::Context::default();
    egui_ctx.set_pixels_per_point(1.0);

    let mut app = QuantaCryptApp::new(db_path, true, Some(ModuleId::I1DoubleSlit));
    match view {
        2 => {
            app.ws.active_module = ModuleId::K1Bb84Qkd;
            app.ws.k1_eve_rate = 0.25;
            app.arena.reset_stream(app.ws.seed, ModuleId::K1Bb84Qkd);
        }
        3 => {
            app.ws.active_module = ModuleId::C3MlKemLattice;
            app.ws.c3_profile = MlKemProfile::MlKem768;
            app.arena.reset_stream(app.ws.seed, ModuleId::C3MlKemLattice);
        }
        4 => {
            app.ws.active_module = ModuleId::Q2CircuitEntanglement;
            app.ws.q2_circuit = QuantumCircuit::preset_ghz(4);
            app.ws.q2_step_cursor = app.ws.q2_circuit.ops.len();
            app.ws.q2_bipartite_cut = 2;
            app.ws.q2_qasm_buffer = app.ws.q2_circuit.to_openqasm();
            app.sync_q2_indices();
            app.arena.reset_stream(app.ws.seed, ModuleId::Q2CircuitEntanglement);
        }
        _ => {
            app.ws.active_module = ModuleId::I1DoubleSlit;
            app.ws.i1_slit_sep = 4.5;
            app.ws.i1_which_path = 0.0;
            app.arena.reset_stream(app.ws.seed, ModuleId::I1DoubleSlit);
        }
    }
    app.stage = AppStage::ModuleWorkspace;
    let steps = if view == 1 { 260 } else { 80 };
    for _ in 0..steps {
        app.arena.step(0.05, &app.ws);
    }

    for frame_idx in 0..4 {
        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width as f32, height as f32),
            )),
            viewports: std::iter::once((
                egui::ViewportId::ROOT,
                egui::ViewportInfo {
                    native_pixels_per_point: Some(1.0),
                    ..Default::default()
                },
            ))
            .collect(),
            time: Some(frame_idx as f64 / 60.0),
            predicted_dt: 1.0 / 60.0,
            ..Default::default()
        };
        let full_output = egui_ctx.run(raw_input, |ctx| {
            app.ui_step(ctx);
        });
        for (id, image_delta) in &full_output.textures_delta.set {
            renderer.update_texture(&device, &queue, *id, image_delta);
        }
        let paint_jobs = egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point: 1.0,
        };
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("quantacrypt-offscreen-encoder"),
        });
        let user_cmd_bufs = renderer.update_buffers(
            &device,
            &queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );
        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("quantacrypt-offscreen-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            let mut render_pass = render_pass.forget_lifetime();
            renderer.render(&mut render_pass, &paint_jobs, &screen_descriptor);
        }
        if frame_idx == 3 {
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &staging,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(bytes_per_row),
                        rows_per_image: Some(height),
                    },
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        }
        queue.submit(user_cmd_bufs.into_iter().chain(std::iter::once(encoder.finish())));
        device.poll(wgpu::Maintain::Wait);
        for id in &full_output.textures_delta.free {
            renderer.free_texture(id);
        }
    }

    let slice = staging.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |res| {
        let _ = tx.send(res);
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv()
        .map_err(|e| format!("GPU readback channel error: {e}"))?
        .map_err(|e| format!("GPU buffer map failed: {e}"))?;
    let mapped = slice.get_mapped_range();
    write_rgba_png(out_path, width, height, &mapped)?;
    drop(mapped);
    staging.unmap();
    Ok(())
}
