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

//! Zero-drift native `egui::Painter` real-time stochastic animation & scientific plotters (`v2.1.0`):
//! - Double-slit 2D Phosphor hit detector screen (`draw_phosphor_screen`)
//! - 3D-projected Bloch Sphere with stochastic quantum-jump walker cloud (`draw_bloch_sphere_cloud`)
//! - Live Photonic Optical Fiber / Waveguide Stream (`draw_photon_stream`)
//! - Multi-qubit Quantum Circuit wire timeline (`draw_circuit_diagram`)
//! - 2D Rotated Surface-Code Checkerboard (`draw_surface_code_grid`)
//! - 8-Node Neutral-Atom Rydberg QAOA Max-Cut Graph (`draw_qaoa_ring_graph`)
//! - Amplitude bar charts, density heatmaps, scientific curves, and 2D Lattice LWE clouds.

use crate::complex::{phase_to_rgb, CMatrix};
use crate::statevector::{GateOp, QuantumCircuit};
use crate::stochastic_sim::FlyingPhoton;
use eframe::egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2};
use std::collections::VecDeque;
use std::f64::consts::PI;

const BG_PANEL: Color32 = Color32::from_rgb(15, 20, 29);
const GRID_LINE: Color32 = Color32::from_rgb(38, 49, 68);
const BORDER_LINE: Color32 = Color32::from_rgb(66, 84, 112);
const TEXT_MAIN: Color32 = Color32::from_rgb(228, 236, 246);
const TEXT_DIM: Color32 = Color32::from_rgb(148, 164, 186);
const ACCENT_CYAN: Color32 = Color32::from_rgb(56, 189, 248);
const ACCENT_AMBER: Color32 = Color32::from_rgb(251, 191, 36);
const ACCENT_EMERALD: Color32 = Color32::from_rgb(52, 211, 153);
const ACCENT_ROSE: Color32 = Color32::from_rgb(251, 113, 133);
const ACCENT_VIOLET: Color32 = Color32::from_rgb(167, 139, 250);

#[inline]
pub fn stroke_rect_box(painter: &Painter, rect: Rect, stroke: Stroke) {
    painter.add(Shape::closed_line(
        vec![
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
            rect.left_bottom(),
        ],
        stroke,
    ));
}

/// Renders a 2D single-quantum phosphor detector screen accumulating `(x_norm, y_norm)` hits (`I1`).
pub fn draw_phosphor_screen(ui: &mut Ui, hits: &VecDeque<(f32, f32)>, height: f32, title: &str) {
    let width = ui.available_width().max(340.0);
    let (rect, _resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 6.0, Color32::from_rgb(8, 13, 22));
    stroke_rect_box(&painter, rect, Stroke::new(1.0, BORDER_LINE));
    painter.text(Pos2::new(rect.left() + 10.0, rect.top() + 6.0), Align2::LEFT_TOP, title, FontId::proportional(12.0), TEXT_MAIN);

    let inner = Rect::from_min_max(Pos2::new(rect.left() + 16.0, rect.top() + 26.0), Pos2::new(rect.right() - 16.0, rect.bottom() - 10.0));
    stroke_rect_box(&painter, inner, Stroke::new(1.0, GRID_LINE));

    let total = hits.len().max(1);
    for (idx, &(xn, yn)) in hits.iter().enumerate() {
        let px = inner.left() + xn.clamp(0.0, 1.0) * inner.width();
        let py = inner.top() + yn.clamp(0.0, 1.0) * inner.height();
        let newest = idx + 8 >= total;
        let col = if newest {
            ACCENT_AMBER
        } else {
            Color32::from_rgba_unmultiplied(56, 189, 248, 190)
        };
        painter.circle_filled(Pos2::new(px, py), if newest { 3.4 } else { 2.2 }, col);
    }
}

/// Renders a `d x d` rotated surface-code lattice (`A1`) highlighting stochastic X (rose) and Z (amber) errors and triggered plaquettes.
pub fn draw_surface_code_grid(ui: &mut Ui, d: usize, data_errors: &[u8], size_px: f32) {
    let (rect, _resp) = ui.allocate_exact_size(Vec2::new(size_px, size_px + 28.0), Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 6.0, BG_PANEL);
    stroke_rect_box(&painter, rect, Stroke::new(1.0, BORDER_LINE));
    painter.text(
        Pos2::new(rect.left() + 10.0, rect.top() + 6.0),
        Align2::LEFT_TOP,
        format!("Rotated Surface Code [[{}, 1, {}]] Stabilizer Lattice", d * d, d),
        FontId::proportional(12.0),
        TEXT_MAIN,
    );

    let grid = Rect::from_min_max(Pos2::new(rect.left() + 20.0, rect.top() + 30.0), Pos2::new(rect.right() - 20.0, rect.bottom() - 14.0));
    let step = grid.width() / (d.max(2) as f32);

    // Plaquette stabilizer parity tiles `(r, c)` over `(d-1) x (d-1)` dual squares
    for r in 0..d.saturating_sub(1) {
        for c in 0..d.saturating_sub(1) {
            let is_x_plaq = (r + c) % 2 == 0;
            let corners = [
                data_errors.get(r * d + c).copied().unwrap_or(0),
                data_errors.get(r * d + c + 1).copied().unwrap_or(0),
                data_errors.get((r + 1) * d + c).copied().unwrap_or(0),
                data_errors.get((r + 1) * d + c + 1).copied().unwrap_or(0),
            ];
            let syndrome = corners
                .iter()
                .filter(|&&e| if is_x_plaq { e == 2 } else { e == 1 })
                .count()
                % 2
                == 1;
            let tile = Rect::from_min_max(
                Pos2::new(grid.left() + (c as f32 + 0.5) * step, grid.top() + (r as f32 + 0.5) * step),
                Pos2::new(grid.left() + (c as f32 + 1.5) * step, grid.top() + (r as f32 + 1.5) * step),
            );
            let fill = if syndrome {
                if is_x_plaq {
                    Color32::from_rgba_unmultiplied(52, 211, 153, 140)
                } else {
                    Color32::from_rgba_unmultiplied(167, 139, 250, 140)
                }
            } else if is_x_plaq {
                Color32::from_rgb(20, 32, 44)
            } else {
                Color32::from_rgb(26, 24, 44)
            };
            painter.rect_filled(tile, 3.0, fill);
            stroke_rect_box(&painter, tile, Stroke::new(1.0, GRID_LINE));
        }
    }

    // Physical data qubits at vertices
    for r in 0..d {
        for c in 0..d {
            let p = Pos2::new(grid.left() + (c as f32 + 0.5) * step, grid.top() + (r as f32 + 0.5) * step);
            let err = data_errors.get(r * d + c).copied().unwrap_or(0);
            let col = match err {
                1 => ACCENT_ROSE,
                2 => ACCENT_AMBER,
                _ => ACCENT_CYAN,
            };
            painter.circle_filled(p, if err > 0 { 6.2 } else { 4.2 }, col);
        }
    }
}

/// Renders an 8-node Rydberg Max-Cut graph (`A3`) coloring spin partitions and cut edges.
pub fn draw_qaoa_ring_graph(ui: &mut Ui, bitmask: u8, size_px: f32) {
    const EDGES: [(usize, usize); 12] = [
        (0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 6),
        (6, 7), (7, 0), (0, 4), (1, 5), (2, 6), (3, 7),
    ];
    let (rect, _resp) = ui.allocate_exact_size(Vec2::new(size_px, size_px + 28.0), Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 6.0, BG_PANEL);
    stroke_rect_box(&painter, rect, Stroke::new(1.0, BORDER_LINE));
    painter.text(Pos2::new(rect.left() + 10.0, rect.top() + 6.0), Align2::LEFT_TOP, "8-Atom Rydberg Graph Partition", FontId::proportional(12.0), TEXT_MAIN);

    let center = Pos2::new(rect.center().x, rect.center().y + 10.0);
    let rad = (size_px * 0.35).max(40.0);
    let node_pos = |i: usize| -> Pos2 {
        let ang = (i as f32) * std::f32::consts::TAU / 8.0 - std::f32::consts::FRAC_PI_2;
        Pos2::new(center.x + rad * ang.cos(), center.y + rad * ang.sin())
    };

    for &(u, v) in &EDGES {
        let cut = ((bitmask >> u) & 1) != ((bitmask >> v) & 1);
        painter.line_segment(
            [node_pos(u), node_pos(v)],
            Stroke::new(if cut { 2.2 } else { 1.0 }, if cut { ACCENT_EMERALD } else { GRID_LINE }),
        );
    }
    for i in 0..8 {
        let bit = (bitmask >> i) & 1;
        let col = if bit == 1 { ACCENT_AMBER } else { ACCENT_CYAN };
        painter.circle_filled(node_pos(i), 8.5, col);
    }
}

pub fn draw_bloch_sphere_cloud(
    ui: &mut Ui,
    size: Vec2,
    mean_bloch: [f64; 3],
    secondary_bloch: Option<([f64; 3], &str)>,
    cloud_walkers: &[[f64; 3]],
    yaw: &mut f32,
    pitch: &mut f32,
    caption: &str,
) {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::drag());
    if resp.dragged() {
        let delta = resp.drag_delta();
        *yaw += delta.x * 0.012;
        *pitch = (*pitch + delta.y * 0.012).clamp(-1.35, 1.35);
    }

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 6.0, BG_PANEL);
    stroke_rect_box(&painter, rect, Stroke::new(1.0, BORDER_LINE));

    let center = Pos2::new(rect.center().x, rect.center().y + 6.0);
    let radius = (rect.width().min(rect.height() - 34.0) * 0.37).max(40.0);
    let (cy, sy, cp, sp) = (yaw.cos(), yaw.sin(), pitch.cos(), pitch.sin());

    let project = |p: [f64; 3]| -> Pos2 {
        let (x, y, z) = (p[0] as f32, p[1] as f32, p[2] as f32);
        let rx = -sy * x + cy * y;
        let ry_depth = cy * x + sy * y;
        Pos2::new(center.x + rx * radius, center.y + (-cp * z + sp * ry_depth) * radius)
    };

    painter.circle_stroke(center, radius, Stroke::new(1.4, Color32::from_rgb(78, 102, 138)));
    for ring in 0..3 {
        let pts: Vec<Pos2> = (0..=64)
            .map(|k| {
                let t = (k as f64) * 2.0 * PI / 64.0;
                project(match ring {
                    0 => [t.cos(), t.sin(), 0.0],
                    1 => [t.cos(), 0.0, t.sin()],
                    _ => [0.0, t.cos(), t.sin()],
                })
            })
            .collect();
        painter.add(Shape::line(pts, Stroke::new(1.0, GRID_LINE)));
    }

    for (neg, pos, ln, lp) in [
        ([-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], "|-⟩", "|+⟩"),
        ([0.0, -1.0, 0.0], [0.0, 1.0, 0.0], "|-i⟩", "|+i⟩"),
        ([0.0, 0.0, -1.0], [0.0, 0.0, 1.0], "|1⟩", "|0⟩"),
    ] {
        let p_neg = project(neg);
        let p_pos = project(pos);
        painter.line_segment([p_neg, p_pos], Stroke::new(1.0, Color32::from_rgb(55, 71, 96)));
        painter.text(p_pos, Align2::CENTER_BOTTOM, lp, FontId::proportional(11.5), TEXT_DIM);
        painter.text(p_neg, Align2::CENTER_TOP, ln, FontId::proportional(11.5), TEXT_DIM);
    }

    for &w in cloud_walkers {
        painter.circle_filled(project(w), 3.0, Color32::from_rgba_unmultiplied(167, 139, 250, 185));
    }

    if let Some((sec_b, sec_lbl)) = secondary_bloch {
        let tip_s = project(sec_b);
        painter.line_segment([center, tip_s], Stroke::new(2.0, ACCENT_AMBER));
        painter.circle_filled(tip_s, 4.5, ACCENT_AMBER);
        painter.text(Pos2::new(tip_s.x + 6.0, tip_s.y - 6.0), Align2::LEFT_BOTTOM, sec_lbl, FontId::proportional(11.0), ACCENT_AMBER);
    }

    let tip = project(mean_bloch);
    painter.line_segment([center, tip], Stroke::new(3.0, ACCENT_CYAN));
    painter.circle_filled(center, 3.0, TEXT_MAIN);
    painter.circle_filled(tip, 5.5, ACCENT_CYAN);

    let norm = (mean_bloch[0] * mean_bloch[0] + mean_bloch[1] * mean_bloch[1] + mean_bloch[2] * mean_bloch[2]).sqrt();
    painter.text(Pos2::new(rect.left() + 10.0, rect.top() + 8.0), Align2::LEFT_TOP, caption, FontId::proportional(12.5), TEXT_MAIN);
    painter.text(
        Pos2::new(rect.left() + 10.0, rect.bottom() - 8.0),
        Align2::LEFT_BOTTOM,
        format!("<r> = ({:+.2}, {:+.2}, {:+.2})  ||<r>|| = {:.3}", mean_bloch[0], mean_bloch[1], mean_bloch[2], norm),
        FontId::monospace(10.5),
        TEXT_DIM,
    );
}

pub fn draw_photon_stream(
    ui: &mut Ui,
    photons: &[FlyingPhoton],
    num_lanes: usize,
    eve_active: bool,
    height: f32,
    title: &str,
) {
    let width = ui.available_width().max(340.0);
    let (rect, _resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 6.0, BG_PANEL);
    stroke_rect_box(&painter, rect, Stroke::new(1.0, BORDER_LINE));
    painter.text(Pos2::new(rect.left() + 10.0, rect.top() + 6.0), Align2::LEFT_TOP, title, FontId::proportional(12.0), TEXT_MAIN);

    let lanes = num_lanes.clamp(1, 8);
    let x_alice = rect.left() + 64.0;
    let x_bob = rect.right() - 64.0;
    let x_eve = 0.5 * (x_alice + x_bob);
    let top_y = rect.top() + 30.0;
    let span_y = (rect.height() - 42.0).max(24.0);

    painter.text(Pos2::new(rect.left() + 12.0, rect.center().y), Align2::LEFT_CENTER, "Source", FontId::monospace(10.5), ACCENT_CYAN);
    painter.text(Pos2::new(rect.right() - 12.0, rect.center().y), Align2::RIGHT_CENTER, "Detector", FontId::monospace(10.5), ACCENT_EMERALD);

    if eve_active {
        painter.line_segment([Pos2::new(x_eve, top_y - 4.0), Pos2::new(x_eve, rect.bottom() - 8.0)], Stroke::new(1.4, ACCENT_ROSE));
        painter.text(Pos2::new(x_eve, rect.bottom() - 4.0), Align2::CENTER_BOTTOM, "Intersector", FontId::monospace(10.0), ACCENT_ROSE);
    }

    let lane_y = |l: usize| -> f32 { top_y + ((l as f32) + 0.5) * (span_y / (lanes as f32)) };
    for l in 0..lanes {
        painter.line_segment([Pos2::new(x_alice, lane_y(l)), Pos2::new(x_bob, lane_y(l))], Stroke::new(1.0, GRID_LINE));
    }

    for p in photons {
        let px = x_alice + p.x_frac.clamp(0.0, 1.0) * (x_bob - x_alice);
        let py = lane_y(p.lane % lanes);
        let col = if p.errored {
            ACCENT_ROSE
        } else if p.intercepted && p.x_frac >= 0.5 {
            ACCENT_AMBER
        } else if p.erased && p.x_frac > 0.82 {
            TEXT_DIM
        } else {
            ACCENT_CYAN
        };
        painter.circle_filled(Pos2::new(px, py), 4.6, col);
    }
}

pub fn draw_circuit_diagram(
    ui: &mut Ui,
    circuit: &QuantumCircuit,
    step_cursor: usize,
    fault_sparks: &[(usize, usize, f32)],
    desired_height: f32,
) {
    let n_q = circuit.num_qubits.max(1);
    let n_ops = circuit.ops.len();
    let width = ui.available_width().max(380.0);
    let height = desired_height.max((n_q as f32) * 36.0 + 40.0);
    let (rect, _resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 6.0, BG_PANEL);
    stroke_rect_box(&painter, rect, Stroke::new(1.0, BORDER_LINE));

    let (left_pad, right_pad, top_pad) = (62.0, 22.0, 26.0);
    let wire_spacing = ((height - top_pad - 16.0) / (n_q as f32)).clamp(22.0, 44.0);
    let wire_y = |q: usize| -> f32 { rect.top() + top_pad + (q as f32 + 0.5) * wire_spacing };

    for q in 0..n_q {
        let y = wire_y(q);
        painter.text(Pos2::new(rect.left() + 10.0, y), Align2::LEFT_CENTER, format!("q[{}] |0⟩", q), FontId::monospace(11.5), TEXT_MAIN);
        painter.line_segment([Pos2::new(rect.left() + left_pad, y), Pos2::new(rect.right() - right_pad, y)], Stroke::new(1.2, Color32::from_rgb(75, 92, 120)));
    }
    if n_ops == 0 {
        return;
    }

    let col_w = ((rect.width() - left_pad - right_pad).max(80.0) / (n_ops as f32)).clamp(26.0, 64.0);
    let clamped_cursor = step_cursor.min(n_ops);

    for (idx, op) in circuit.ops.iter().enumerate() {
        let cx = rect.left() + left_pad + (idx as f32 + 0.5) * col_w;
        let box_stroke = if idx < clamped_cursor { ACCENT_CYAN } else { BORDER_LINE };
        match *op {
            GateOp::Cnot { control, target } => {
                let (yc, yt) = (wire_y(control), wire_y(target));
                painter.line_segment([Pos2::new(cx, yc), Pos2::new(cx, yt)], Stroke::new(2.0, box_stroke));
                painter.circle_filled(Pos2::new(cx, yc), 4.5, box_stroke);
                painter.circle_stroke(Pos2::new(cx, yt), 9.0, Stroke::new(2.0, box_stroke));
            }
            GateOp::Cz { control, target } | GateOp::CPhase { control, target, .. } => {
                let (yc, yt) = (wire_y(control), wire_y(target));
                painter.line_segment([Pos2::new(cx, yc), Pos2::new(cx, yt)], Stroke::new(2.0, ACCENT_VIOLET));
                painter.circle_filled(Pos2::new(cx, yc), 4.5, ACCENT_VIOLET);
                painter.circle_filled(Pos2::new(cx, yt), 4.5, ACCENT_VIOLET);
            }
            GateOp::Swap(a, b) => {
                painter.line_segment([Pos2::new(cx, wire_y(a)), Pos2::new(cx, wire_y(b))], Stroke::new(1.8, ACCENT_AMBER));
            }
            _ => {
                let g_rect = Rect::from_center_size(Pos2::new(cx, wire_y(op.max_qubit_index())), Vec2::new((col_w - 6.0).clamp(22.0, 48.0), 22.0));
                painter.rect_filled(g_rect, 4.0, Color32::from_rgb(30, 64, 110));
                stroke_rect_box(&painter, g_rect, Stroke::new(1.2, box_stroke));
                painter.text(g_rect.center(), Align2::CENTER_CENTER, op.label(), FontId::monospace(10.0), TEXT_MAIN);
            }
        }
    }

    for &(q, col, ttl) in fault_sparks {
        if q < n_q && col < n_ops {
            let cx = rect.left() + left_pad + (col as f32 + 0.5) * col_w;
            painter.circle_stroke(Pos2::new(cx, wire_y(q)), 12.0 * (1.2 - ttl), Stroke::new(2.0, ACCENT_ROSE));
        }
    }
}

pub fn draw_amplitude_phase_bars(
    ui: &mut Ui,
    labels: &[String],
    probs: &[f64],
    phases: Option<&[f64]>,
    height: f32,
    title: &str,
) {
    let width = ui.available_width().max(320.0);
    let (rect, _resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 6.0, BG_PANEL);
    stroke_rect_box(&painter, rect, Stroke::new(1.0, BORDER_LINE));
    painter.text(Pos2::new(rect.left() + 10.0, rect.top() + 7.0), Align2::LEFT_TOP, title, FontId::proportional(12.0), TEXT_MAIN);

    let n = probs.len().min(64).max(1);
    let plot_rect = Rect::from_min_max(Pos2::new(rect.left() + 44.0, rect.top() + 26.0), Pos2::new(rect.right() - 14.0, rect.bottom() - 26.0));
    let max_p = probs.iter().copied().fold(0.05_f64, f64::max);

    for k in 0..=4 {
        let y = plot_rect.bottom() - (k as f32) * 0.25 * plot_rect.height();
        painter.line_segment([Pos2::new(plot_rect.left(), y), Pos2::new(plot_rect.right(), y)], Stroke::new(1.0, GRID_LINE));
    }

    let slot_w = plot_rect.width() / (n as f32);
    let bar_w = (slot_w * 0.74).clamp(2.0, 42.0);
    let stride = if n <= 16 { 1 } else if n <= 32 { 2 } else { 4 };

    for i in 0..n {
        let p = (probs[i] / max_p).clamp(0.0, 1.0) as f32;
        let x_center = plot_rect.left() + (i as f32 + 0.5) * slot_w;
        let bar_h = p * plot_rect.height();
        let bar_rect = Rect::from_min_max(Pos2::new(x_center - bar_w * 0.5, plot_rect.bottom() - bar_h), Pos2::new(x_center + bar_w * 0.5, plot_rect.bottom()));
        let fill = if let Some(ph) = phases {
            let [r, g, b] = phase_to_rgb(ph.get(i).copied().unwrap_or(0.0));
            Color32::from_rgb(r, g, b)
        } else {
            ACCENT_CYAN
        };
        if bar_h > 0.5 {
            painter.rect_filled(bar_rect, 2.0, fill);
        }
        if i % stride == 0 {
            if let Some(lbl) = labels.get(i) {
                painter.text(Pos2::new(x_center, plot_rect.bottom() + 4.0), Align2::CENTER_TOP, lbl, FontId::monospace(9.5), TEXT_DIM);
            }
        }
    }
}

pub fn draw_density_heatmap(ui: &mut Ui, rho: &CMatrix, size_px: f32, title: &str) {
    let dim = rho.dim.clamp(1, 16);
    let (rect, _resp) = ui.allocate_exact_size(Vec2::new(size_px, size_px + 28.0), Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 6.0, BG_PANEL);
    stroke_rect_box(&painter, rect, Stroke::new(1.0, BORDER_LINE));
    painter.text(Pos2::new(rect.left() + 10.0, rect.top() + 6.0), Align2::LEFT_TOP, title, FontId::proportional(12.0), TEXT_MAIN);

    let grid_rect = Rect::from_min_max(Pos2::new(rect.left() + 12.0, rect.top() + 26.0), Pos2::new(rect.right() - 12.0, rect.bottom() - 10.0));
    let (cell_w, cell_h) = (grid_rect.width() / (dim as f32), grid_rect.height() / (dim as f32));
    for r in 0..dim {
        for c in 0..dim {
            let z = rho.get(r, c);
            let mag = z.norm().clamp(0.0, 1.0) as f32;
            let [pr, pg, pb] = phase_to_rgb(z.arg());
            let fill = Color32::from_rgb(
                (18.0 + (pr as f32 - 18.0) * mag) as u8,
                (24.0 + (pg as f32 - 24.0) * mag) as u8,
                (34.0 + (pb as f32 - 34.0) * mag) as u8,
            );
            let c_rect = Rect::from_min_max(
                Pos2::new(grid_rect.left() + (c as f32) * cell_w + 1.0, grid_rect.top() + (r as f32) * cell_h + 1.0),
                Pos2::new(grid_rect.left() + ((c + 1) as f32) * cell_w - 1.0, grid_rect.top() + ((r + 1) as f32) * cell_h - 1.0),
            );
            painter.rect_filled(c_rect, 2.0, fill);
        }
    }
}

pub struct PlotSeries<'a> {
    pub label: &'a str,
    pub points: &'a [(f64, f64)],
    pub color: Color32,
}

pub struct PlotHThreshold<'a> {
    pub y: f64,
    pub label: &'a str,
    pub color: Color32,
}

pub fn draw_scientific_plot(
    ui: &mut Ui,
    title: &str,
    _x_label: &str,
    _y_label: &str,
    x_range: (f64, f64),
    y_range: (f64, f64),
    series: &[PlotSeries<'_>],
    thresholds: &[PlotHThreshold<'_>],
    height: f32,
) {
    let width = ui.available_width().max(320.0);
    let (rect, _resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 6.0, BG_PANEL);
    stroke_rect_box(&painter, rect, Stroke::new(1.0, BORDER_LINE));
    painter.text(Pos2::new(rect.left() + 10.0, rect.top() + 6.0), Align2::LEFT_TOP, title, FontId::proportional(12.0), TEXT_MAIN);

    let plot_rect = Rect::from_min_max(Pos2::new(rect.left() + 54.0, rect.top() + 28.0), Pos2::new(rect.right() - 22.0, rect.bottom() - 24.0));
    stroke_rect_box(&painter, plot_rect, Stroke::new(1.0, GRID_LINE));

    let dx = (x_range.1 - x_range.0).abs().max(1e-9);
    let dy = (y_range.1 - y_range.0).abs().max(1e-9);
    let to_screen = |x: f64, y: f64| -> Pos2 {
        let fx = ((x - x_range.0) / dx).clamp(0.0, 1.0) as f32;
        let fy = ((y - y_range.0) / dy).clamp(0.0, 1.0) as f32;
        Pos2::new(plot_rect.left() + fx * plot_rect.width(), plot_rect.bottom() - fy * plot_rect.height())
    };

    for g in 0..=4 {
        let frac = (g as f64) * 0.25;
        let yv = y_range.0 + frac * dy;
        let py = plot_rect.bottom() - (frac as f32) * plot_rect.height();
        painter.line_segment([Pos2::new(plot_rect.left(), py), Pos2::new(plot_rect.right(), py)], Stroke::new(1.0, GRID_LINE));
        painter.text(Pos2::new(plot_rect.left() - 6.0, py), Align2::RIGHT_CENTER, format!("{:.2}", yv), FontId::monospace(9.5), TEXT_DIM);
    }

    for th in thresholds {
        if th.y >= y_range.0 && th.y <= y_range.1 {
            let (p_l, p_r) = (to_screen(x_range.0, th.y), to_screen(x_range.1, th.y));
            painter.line_segment([p_l, p_r], Stroke::new(1.3, th.color));
            painter.text(Pos2::new(p_r.x - 4.0, p_r.y - 3.0), Align2::RIGHT_BOTTOM, th.label, FontId::proportional(10.0), th.color);
        }
    }

    let mut legend_x = rect.right() - 14.0;
    for s in series.iter().rev() {
        if s.points.len() >= 2 {
            let pts: Vec<Pos2> = s.points.iter().map(|&(x, y)| to_screen(x, y)).collect();
            painter.add(Shape::line(pts, Stroke::new(2.0, s.color)));
        }
        painter.text(Pos2::new(legend_x, rect.top() + 7.0), Align2::RIGHT_TOP, format!("■ {}", s.label), FontId::proportional(10.5), s.color);
        legend_x -= (s.label.len() as f32) * 6.5 + 24.0;
    }
}

pub fn draw_lattice_2d_cloud(
    ui: &mut Ui,
    points: &[[f64; 2]],
    b1: [f64; 2],
    b2: [f64; 2],
    exact_target: [f64; 2],
    lwe_cloud: &[[f64; 2]],
    size: Vec2,
) {
    let (rect, _resp) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 6.0, BG_PANEL);
    stroke_rect_box(&painter, rect, Stroke::new(1.0, BORDER_LINE));
    painter.text(Pos2::new(rect.left() + 10.0, rect.top() + 6.0), Align2::LEFT_TOP, "2D Lattice LWE Scatter Cloud: Exact A*s (Cyan) & Noisy t = A*s + e (Rose)", FontId::proportional(12.0), TEXT_MAIN);

    let center = Pos2::new(rect.center().x, rect.center().y + 10.0);
    let scale = (rect.width().min(rect.height()) / 42.0).max(4.5);
    let to_px = |v: [f64; 2]| -> Pos2 { Pos2::new(center.x + (v[0] as f32) * scale, center.y - (v[1] as f32) * scale) };

    for &pt in points {
        let p = to_px(pt);
        if rect.shrink(14.0).contains(p) {
            painter.circle_filled(p, 2.5, Color32::from_rgb(84, 106, 140));
        }
    }
    let orig = to_px([0.0, 0.0]);
    painter.line_segment([orig, to_px(b1)], Stroke::new(2.0, ACCENT_EMERALD));
    painter.line_segment([orig, to_px(b2)], Stroke::new(2.0, ACCENT_VIOLET));
    for &sample in lwe_cloud {
        painter.circle_filled(to_px(sample), 3.2, Color32::from_rgba_unmultiplied(251, 113, 133, 175));
    }
    painter.circle_filled(to_px(exact_target), 5.2, ACCENT_CYAN);
}

/// Renders an `N_hops` entanglement-swapping quantum repeater chain (`Q5`) with `N_hops + 1`
/// SPDC crystals, `N_hops` intermediate Bell-state analyzers, and an end-to-end heralded entanglement arc.
pub fn draw_repeater_chain(
    ui: &mut Ui,
    hops: usize,
    swap_flags: &[bool],
    phase: f32,
    height: f32,
) {
    let width = ui.available_width().max(360.0);
    let (rect, _resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 6.0, BG_PANEL);
    stroke_rect_box(&painter, rect, Stroke::new(1.0, BORDER_LINE));
    painter.text(
        Pos2::new(rect.left() + 10.0, rect.top() + 6.0),
        Align2::LEFT_TOP,
        "Multi-Hop Entanglement Swapping & Linear-Optical BSM Chain",
        FontId::proportional(12.0),
        TEXT_MAIN,
    );

    let n_seg = hops.clamp(1, 4) + 1;
    let x_left = rect.left() + 48.0;
    let x_right = rect.right() - 48.0;
    let cy = rect.center().y + 14.0;
    let seg_w = (x_right - x_left) / (n_seg as f32);

    painter.line_segment(
        [Pos2::new(x_left, cy), Pos2::new(x_right, cy)],
        Stroke::new(1.4, GRID_LINE),
    );
    painter.circle_filled(Pos2::new(x_left, cy), 8.0, ACCENT_CYAN);
    painter.text(Pos2::new(x_left, cy + 14.0), Align2::CENTER_TOP, "Alice", FontId::monospace(10.5), ACCENT_CYAN);
    painter.circle_filled(Pos2::new(x_right, cy), 8.0, ACCENT_CYAN);
    painter.text(Pos2::new(x_right, cy + 14.0), Align2::CENTER_TOP, "Bob", FontId::monospace(10.5), ACCENT_CYAN);

    for s in 0..n_seg {
        let sx = x_left + ((s as f32) + 0.5) * seg_w;
        let lx = x_left + (s as f32) * seg_w;
        let rx = x_left + ((s as f32) + 1.0) * seg_w;
        let u = phase.rem_euclid(1.0);
        let p1 = sx - u * (sx - lx - 12.0);
        let p2 = sx + u * (rx - sx - 12.0);
        painter.circle_filled(Pos2::new(sx, cy), 5.0, ACCENT_VIOLET);
        painter.text(Pos2::new(sx, cy - 12.0), Align2::CENTER_BOTTOM, "SPDC", FontId::monospace(9.5), ACCENT_VIOLET);
        painter.circle_filled(Pos2::new(p1, cy), 3.6, ACCENT_AMBER);
        painter.circle_filled(Pos2::new(p2, cy), 3.6, ACCENT_AMBER);
    }

    let mut all_heralded = !swap_flags.is_empty();
    for h in 0..hops.clamp(1, 4) {
        let bx = x_left + ((h + 1) as f32) * seg_w;
        let ok = swap_flags.get(h).copied().unwrap_or(false);
        if !ok {
            all_heralded = false;
        }
        let b_rect = Rect::from_center_size(Pos2::new(bx, cy), Vec2::new(34.0, 22.0));
        let col = if ok { ACCENT_EMERALD } else { ACCENT_ROSE };
        painter.rect_filled(b_rect, 4.0, Color32::from_rgb(20, 30, 44));
        stroke_rect_box(&painter, b_rect, Stroke::new(1.5, col));
        painter.text(b_rect.center(), Align2::CENTER_CENTER, format!("B{}", h + 1), FontId::monospace(10.0), col);
    }

    if all_heralded {
        let arc_pts: Vec<Pos2> = (0..=36)
            .map(|k| {
                let t = (k as f32) / 36.0;
                let x = x_left + t * (x_right - x_left);
                let y = cy - 42.0 * (std::f32::consts::PI * t).sin();
                Pos2::new(x, y)
            })
            .collect();
        painter.add(Shape::line(arc_pts, Stroke::new(2.2, ACCENT_EMERALD)));
    }
}
