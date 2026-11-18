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

//! Deterministic procedural RGBA application icon for QuantaCrypt Studio.

use eframe::egui::IconData;

fn blend_pixel(dst: &mut [f64; 4], r: f64, g: f64, b: f64, a: f64) {
    let alpha = a.clamp(0.0, 1.0);
    if alpha <= 0.0 {
        return;
    }
    let inv = 1.0 - alpha;
    dst[0] = dst[0] * inv + r * alpha;
    dst[1] = dst[1] * inv + g * alpha;
    dst[2] = dst[2] * inv + b * alpha;
    dst[3] = (dst[3] + alpha * (1.0 - dst[3])).clamp(0.0, 1.0);
}

fn sample_icon_pixel(u: f64, v: f64) -> [f64; 4] {
    // Rounded-square SDF on [-1, 1]^2
    let half = 0.88_f64;
    let rad = 0.28_f64;
    let qx = u.abs() - (half - rad);
    let qy = v.abs() - (half - rad);
    let sdf = qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - rad;
    if sdf > 0.02 {
        return [0.0, 0.0, 0.0, 0.0];
    }

    let badge_cov = ((0.02 - sdf) / 0.04).clamp(0.0, 1.0);
    let mut px = [15.0 / 255.0, 20.0 / 255.0, 32.0 / 255.0, badge_cov];

    // Subtle radial glow in center
    let r_center = u.hypot(v);
    let glow = (-r_center * r_center * 3.5).exp() * 0.28;
    blend_pixel(&mut px, 56.0 / 255.0, 189.0 / 255.0, 248.0 / 255.0, glow * badge_cov);

    // Outer metallic gradient border
    if sdf > -0.07 {
        let t = ((u + v) * 0.5 + 0.5).clamp(0.0, 1.0);
        let br = (56.0 * (1.0 - t) + 167.0 * t) / 255.0;
        let bg = (189.0 * (1.0 - t) + 139.0 * t) / 255.0;
        let bb = (248.0 * (1.0 - t) + 250.0 * t) / 255.0;
        let rim = ((sdf + 0.07) / 0.03).clamp(0.0, 1.0) * 0.85;
        blend_pixel(&mut px, br, bg, bb, rim * badge_cov);
    }

    // Two tilted elliptical quantum orbital rings (+35 deg cyan, -35 deg violet)
    let angle = 35.0_f64.to_radians();
    let (ca, sa) = (angle.cos(), angle.sin());
    for (sign, col) in [
        (1.0_f64, [56.0 / 255.0, 189.0 / 255.0, 248.0 / 255.0]),
        (-1.0_f64, [167.0 / 255.0, 139.0 / 255.0, 250.0 / 255.0]),
    ] {
        let ru = ca * u + sign * sa * v;
        let rv = -sign * sa * u + ca * v;
        let ell = ((ru / 0.64).powi(2) + (rv / 0.25).powi(2)).sqrt();
        let d_ring = (ell - 1.0).abs() * 0.25;
        if d_ring < 0.045 {
            let w = (1.0 - d_ring / 0.045).clamp(0.0, 1.0);
            blend_pixel(&mut px, col[0], col[1], col[2], w * 0.95 * badge_cov);
        }
    }

    // Central amber cryptographic qubit core + emerald outer nodes
    let d_core = u.hypot(v);
    if d_core < 0.19 {
        let w = ((0.19 - d_core) / 0.04).clamp(0.0, 1.0);
        blend_pixel(&mut px, 251.0 / 255.0, 191.0 / 255.0, 36.0 / 255.0, w * badge_cov);
    }
    // Inner keyhole slit motif inside core
    let in_key = (u.abs() < 0.038 && v > -0.02 && v < 0.11) || u.hypot(v + 0.03) < 0.055;
    if in_key {
        blend_pixel(&mut px, 15.0 / 255.0, 20.0 / 255.0, 32.0 / 255.0, 0.92 * badge_cov);
    }

    // Two bright emerald qubit nodes on orbital tips
    for (nx, ny) in [(0.48_f64, -0.34_f64), (-0.48_f64, 0.34_f64)] {
        let d = (u - nx).hypot(v - ny);
        if d < 0.105 {
            let w = ((0.105 - d) / 0.03).clamp(0.0, 1.0);
            blend_pixel(&mut px, 52.0 / 255.0, 211.0 / 255.0, 153.0 / 255.0, w * badge_cov);
        }
    }

    px
}

/// Renders an anti-aliased `size x size` RGBA8 byte buffer for the QuantaCrypt Studio icon.
pub fn app_icon_rgba(size: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    let sub = 2;
    let inv_n = 1.0 / (size as f64);
    let inv_s = 1.0 / ((sub * sub) as f64);
    for y in 0..size {
        for x in 0..size {
            let mut acc = [0.0_f64; 4];
            for sy in 0..sub {
                for sx in 0..sub {
                    let fx = (x as f64 + (sx as f64 + 0.5) / (sub as f64)) * inv_n;
                    let fy = (y as f64 + (sy as f64 + 0.5) / (sub as f64)) * inv_n;
                    let s = sample_icon_pixel(fx * 2.0 - 1.0, fy * 2.0 - 1.0);
                    acc[0] += s[0] * inv_s;
                    acc[1] += s[1] * inv_s;
                    acc[2] += s[2] * inv_s;
                    acc[3] += s[3] * inv_s;
                }
            }
            out.push((acc[0].clamp(0.0, 1.0) * 255.0).round() as u8);
            out.push((acc[1].clamp(0.0, 1.0) * 255.0).round() as u8);
            out.push((acc[2].clamp(0.0, 1.0) * 255.0).round() as u8);
            out.push((acc[3].clamp(0.0, 1.0) * 255.0).round() as u8);
        }
    }
    out
}

/// Builds the `egui::IconData` passed to `ViewportBuilder::with_icon`.
pub fn app_icon_data() -> IconData {
    let size = 64_u32;
    IconData {
        rgba: app_icon_rgba(size),
        width: size,
        height: size,
    }
}
