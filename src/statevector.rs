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

//! Multi-qubit coherent statevector simulator (1..=12 qubits), step-by-step circuit execution,
//! reduced Bloch vectors, bipartite entanglement entropy, and OpenQASM 2.0 parser/exporter.

use crate::complex::{rho_to_bloch, CMatrix, Complex64};
use crate::rng_sha256::DeterministicRng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::f64::consts::{FRAC_1_SQRT_2, PI};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum GateOp {
    Id(usize),
    H(usize),
    X(usize),
    Y(usize),
    Z(usize),
    S(usize),
    Sdg(usize),
    T(usize),
    Tdg(usize),
    Rx { target: usize, theta: f64 },
    Ry { target: usize, theta: f64 },
    Rz { target: usize, theta: f64 },
    Phase { target: usize, lambda: f64 },
    Cnot { control: usize, target: usize },
    Cz { control: usize, target: usize },
    CPhase { control: usize, target: usize, lambda: f64 },
    Swap(usize, usize),
    Toffoli { c1: usize, c2: usize, target: usize },
    Measure(usize),
}

impl GateOp {
    pub fn label(&self) -> String {
        match self {
            Self::Id(_) => "I".to_string(),
            Self::H(_) => "H".to_string(),
            Self::X(_) => "X".to_string(),
            Self::Y(_) => "Y".to_string(),
            Self::Z(_) => "Z".to_string(),
            Self::S(_) => "S".to_string(),
            Self::Sdg(_) => "S†".to_string(),
            Self::T(_) => "T".to_string(),
            Self::Tdg(_) => "T†".to_string(),
            Self::Rx { theta, .. } => format!("Rx({:.2})", theta),
            Self::Ry { theta, .. } => format!("Ry({:.2})", theta),
            Self::Rz { theta, .. } => format!("Rz({:.2})", theta),
            Self::Phase { lambda, .. } => format!("P({:.2})", lambda),
            Self::Cnot { .. } => "CNOT".to_string(),
            Self::Cz { .. } => "CZ".to_string(),
            Self::CPhase { lambda, .. } => format!("CP({:.2})", lambda),
            Self::Swap(_, _) => "SWAP".to_string(),
            Self::Toffoli { .. } => "CCX".to_string(),
            Self::Measure(_) => "M".to_string(),
        }
    }

    pub fn max_qubit_index(&self) -> usize {
        match *self {
            Self::Id(q)
            | Self::H(q)
            | Self::X(q)
            | Self::Y(q)
            | Self::Z(q)
            | Self::S(q)
            | Self::Sdg(q)
            | Self::T(q)
            | Self::Tdg(q)
            | Self::Rx { target: q, .. }
            | Self::Ry { target: q, .. }
            | Self::Rz { target: q, .. }
            | Self::Phase { target: q, .. }
            | Self::Measure(q) => q,
            Self::Cnot { control, target }
            | Self::Cz { control, target }
            | Self::CPhase { control, target, .. }
            | Self::Swap(control, target) => control.max(target),
            Self::Toffoli { c1, c2, target } => c1.max(c2).max(target),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuantumCircuit {
    pub num_qubits: usize,
    pub ops: Vec<GateOp>,
}

impl QuantumCircuit {
    pub fn new(num_qubits: usize) -> Self {
        Self {
            num_qubits: num_qubits.clamp(1, 12),
            ops: Vec::new(),
        }
    }

    pub fn preset_bell_phi_plus() -> Self {
        Self {
            num_qubits: 2,
            ops: vec![GateOp::H(0), GateOp::Cnot { control: 0, target: 1 }],
        }
    }

    pub fn preset_ghz(num_qubits: usize) -> Self {
        let n = num_qubits.clamp(2, 8);
        let mut ops = vec![GateOp::H(0)];
        for q in 1..n {
            ops.push(GateOp::Cnot {
                control: 0,
                target: q,
            });
        }
        Self { num_qubits: n, ops }
    }

    pub fn preset_qft(num_qubits: usize) -> Self {
        let n = num_qubits.clamp(2, 6);
        let mut ops = Vec::new();
        for i in 0..n {
            ops.push(GateOp::H(i));
            for j in (i + 1)..n {
                let k = (j - i + 1) as u32;
                let lambda = 2.0 * PI / ((1u32 << k) as f64);
                ops.push(GateOp::CPhase {
                    control: j,
                    target: i,
                    lambda,
                });
            }
        }
        for i in 0..(n / 2) {
            ops.push(GateOp::Swap(i, n - 1 - i));
        }
        Self { num_qubits: n, ops }
    }

    /// Exports the circuit as standard OpenQASM 2.0 text.
    pub fn to_openqasm(&self) -> String {
        let mut out = String::new();
        out.push_str("OPENQASM 2.0;\n");
        out.push_str("include \"qelib1.inc\";\n");
        out.push_str(&format!("qreg q[{}];\n", self.num_qubits));
        out.push_str(&format!("creg c[{}];\n\n", self.num_qubits));
        for op in &self.ops {
            let line = match *op {
                GateOp::Id(q) => format!("id q[{}];", q),
                GateOp::H(q) => format!("h q[{}];", q),
                GateOp::X(q) => format!("x q[{}];", q),
                GateOp::Y(q) => format!("y q[{}];", q),
                GateOp::Z(q) => format!("z q[{}];", q),
                GateOp::S(q) => format!("s q[{}];", q),
                GateOp::Sdg(q) => format!("sdg q[{}];", q),
                GateOp::T(q) => format!("t q[{}];", q),
                GateOp::Tdg(q) => format!("tdg q[{}];", q),
                GateOp::Rx { target, theta } => format!("rx({:.6}) q[{}];", theta, target),
                GateOp::Ry { target, theta } => format!("ry({:.6}) q[{}];", theta, target),
                GateOp::Rz { target, theta } => format!("rz({:.6}) q[{}];", theta, target),
                GateOp::Phase { target, lambda } => format!("p({:.6}) q[{}];", lambda, target),
                GateOp::Cnot { control, target } => format!("cx q[{}],q[{}];", control, target),
                GateOp::Cz { control, target } => format!("cz q[{}],q[{}];", control, target),
                GateOp::CPhase {
                    control,
                    target,
                    lambda,
                } => format!("cp({:.6}) q[{}],q[{}];", lambda, control, target),
                GateOp::Swap(a, b) => format!("swap q[{}],q[{}];", a, b),
                GateOp::Toffoli { c1, c2, target } => {
                    format!("ccx q[{}],q[{}],q[{}];", c1, c2, target)
                }
                GateOp::Measure(q) => format!("measure q[{}] -> c[{}];", q, q),
            };
            out.push_str(&line);
            out.push('\n');
        }
        out
    }

    /// Parses a subset of OpenQASM 2.0 (qreg, h, x, y, z, s, sdg, t, tdg, rx, ry, rz, p/u1, cx, cz, cp, swap, ccx, measure).
    pub fn from_openqasm(src: &str) -> Result<Self, String> {
        let mut num_qubits: Option<usize> = None;
        let mut ops = Vec::new();

        for (line_idx, raw_line) in src.lines().enumerate() {
            let line_no = line_idx + 1;
            let stripped = match raw_line.find("//") {
                Some(pos) => &raw_line[..pos],
                None => raw_line,
            };
            for stmt in stripped.split(';') {
                let s = stmt.trim();
                if s.is_empty()
                    || s.starts_with("OPENQASM")
                    || s.starts_with("include")
                    || s.starts_with("creg")
                    || s.starts_with("barrier")
                {
                    continue;
                }
                if let Some(rest) = s.strip_prefix("qreg") {
                    let q_idx = parse_bracket_index(rest.trim())
                        .map_err(|e| format!("Line {}: invalid qreg '{}': {}", line_no, s, e))?;
                    if !(1..=12).contains(&q_idx) {
                        return Err(format!("Line {}: qreg size {} must be in 1..=12", line_no, q_idx));
                    }
                    num_qubits = Some(q_idx);
                    continue;
                }
                if let Some(rest) = s.strip_prefix("measure") {
                    let parts: Vec<&str> = rest.split("->").collect();
                    if parts.is_empty() {
                        return Err(format!("Line {}: malformed measure statement", line_no));
                    }
                    let q = parse_bracket_index(parts[0].trim())
                        .map_err(|e| format!("Line {}: {}", line_no, e))?;
                    ops.push(GateOp::Measure(q));
                    continue;
                }

                // Split gate head (possibly with angle `(expr)`) from arguments
                let (head, args_str) = split_gate_and_args(s)
                    .ok_or_else(|| format!("Line {}: unrecognized statement '{}'", line_no, s))?;
                let args: Result<Vec<usize>, String> = args_str
                    .split(',')
                    .map(|tok| parse_bracket_index(tok.trim()))
                    .collect();
                let qargs = args.map_err(|e| format!("Line {}: {}", line_no, e))?;

                let (gname, param_opt) = if let Some(open_p) = head.find('(') {
                    let close_p = head
                        .rfind(')')
                        .ok_or_else(|| format!("Line {}: missing ')' in '{}'", line_no, head))?;
                    let name = head[..open_p].trim().to_ascii_lowercase();
                    let expr = head[open_p + 1..close_p].trim();
                    let val = eval_angle_expr(expr)
                        .map_err(|e| format!("Line {}: angle '{}': {}", line_no, expr, e))?;
                    (name, Some(val))
                } else {
                    (head.trim().to_ascii_lowercase(), None)
                };

                let op = match (gname.as_str(), param_opt, qargs.as_slice()) {
                    ("id", None, &[q]) => GateOp::Id(q),
                    ("h", None, &[q]) => GateOp::H(q),
                    ("x", None, &[q]) => GateOp::X(q),
                    ("y", None, &[q]) => GateOp::Y(q),
                    ("z", None, &[q]) => GateOp::Z(q),
                    ("s", None, &[q]) => GateOp::S(q),
                    ("sdg", None, &[q]) => GateOp::Sdg(q),
                    ("t", None, &[q]) => GateOp::T(q),
                    ("tdg", None, &[q]) => GateOp::Tdg(q),
                    ("rx", Some(theta), &[q]) => GateOp::Rx { target: q, theta },
                    ("ry", Some(theta), &[q]) => GateOp::Ry { target: q, theta },
                    ("rz", Some(theta), &[q]) => GateOp::Rz { target: q, theta },
                    ("p" | "u1", Some(lambda), &[q]) => GateOp::Phase { target: q, lambda },
                    ("cx" | "cnot", None, &[c, t]) => {
                        if c == t {
                            return Err(format!(
                                "Line {}: multi-qubit gate '{}' requires distinct control and target (got q[{}])",
                                line_no, gname, c
                            ));
                        }
                        GateOp::Cnot {
                            control: c,
                            target: t,
                        }
                    }
                    ("cz", None, &[c, t]) => {
                        if c == t {
                            return Err(format!(
                                "Line {}: multi-qubit gate '{}' requires distinct control and target (got q[{}])",
                                line_no, gname, c
                            ));
                        }
                        GateOp::Cz {
                            control: c,
                            target: t,
                        }
                    }
                    ("cp" | "cu1", Some(lambda), &[c, t]) => {
                        if c == t {
                            return Err(format!(
                                "Line {}: multi-qubit gate '{}' requires distinct control and target (got q[{}])",
                                line_no, gname, c
                            ));
                        }
                        GateOp::CPhase {
                            control: c,
                            target: t,
                            lambda,
                        }
                    }
                    ("swap", None, &[a, b]) => {
                        if a == b {
                            return Err(format!(
                                "Line {}: swap gate requires distinct qubits (got q[{}])",
                                line_no, a
                            ));
                        }
                        GateOp::Swap(a, b)
                    }
                    ("ccx" | "toffoli", None, &[c1, c2, t]) => {
                        if c1 == c2 || c1 == t || c2 == t {
                            return Err(format!(
                                "Line {}: Toffoli gate '{}' requires three distinct qubits (got q[{}], q[{}], q[{}])",
                                line_no, gname, c1, c2, t
                            ));
                        }
                        GateOp::Toffoli {
                            c1,
                            c2,
                            target: t,
                        }
                    }
                    _ => {
                        return Err(format!(
                            "Line {}: unsupported gate or arity '{}'",
                            line_no, s
                        ))
                    }
                };
                ops.push(op);
            }
        }

        let max_qubit = ops.iter().map(GateOp::max_qubit_index).max();
        if let Some(declared_n) = num_qubits {
            if let Some(m_idx) = max_qubit {
                if m_idx >= declared_n {
                    return Err(format!(
                        "Qubit index q[{}] exceeds declared qreg q[{}]",
                        m_idx, declared_n
                    ));
                }
            }
        }
        let inferred = max_qubit.map(|m| m.saturating_add(1)).unwrap_or(2);
        let final_n = num_qubits.unwrap_or(inferred);
        if final_n > 12 {
            return Err(format!("Circuit requires {} qubits (maximum supported is 12)", final_n));
        }
        Ok(Self {
            num_qubits: final_n,
            ops,
        })
    }
}

fn split_gate_and_args(stmt: &str) -> Option<(&str, &str)> {
    if let Some(close_paren) = stmt.rfind(')') {
        let rest = stmt[close_paren + 1..].trim();
        if rest.is_empty() {
            None
        } else {
            Some((&stmt[..=close_paren], rest))
        }
    } else {
        let mut iter = stmt.splitn(2, char::is_whitespace);
        let g = iter.next()?.trim();
        let rest = iter.next()?.trim();
        Some((g, rest))
    }
}

fn parse_bracket_index(token: &str) -> Result<usize, String> {
    let lb = token
        .find('[')
        .ok_or_else(|| format!("expected register[index], found '{}'", token))?;
    let rb = token
        .find(']')
        .ok_or_else(|| format!("missing ']' in '{}'", token))?;
    if rb <= lb + 1 {
        return Err(format!("empty index in '{}'", token));
    }
    token[lb + 1..rb]
        .trim()
        .parse::<usize>()
        .map_err(|_| format!("non-integer index in '{}'", token))
}

struct AngleParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> AngleParser<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            bytes: src.as_bytes(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn parse_expr(&mut self) -> Result<f64, String> {
        let mut lhs = self.parse_term()?;
        while let Some(op) = self.peek() {
            if op == b'+' || op == b'-' {
                self.pos += 1;
                let rhs = self.parse_term()?;
                lhs = if op == b'+' { lhs + rhs } else { lhs - rhs };
                if !lhs.is_finite() {
                    return Err("non-finite angle expression".to_string());
                }
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn parse_term(&mut self) -> Result<f64, String> {
        let mut lhs = self.parse_factor()?;
        while let Some(op) = self.peek() {
            if op == b'*' || op == b'/' {
                self.pos += 1;
                let rhs = self.parse_factor()?;
                if op == b'*' {
                    lhs *= rhs;
                } else {
                    if rhs.abs() < 1e-15 {
                        return Err("division by zero in angle".to_string());
                    }
                    lhs /= rhs;
                }
                if !lhs.is_finite() {
                    return Err("non-finite angle expression".to_string());
                }
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn parse_factor(&mut self) -> Result<f64, String> {
        match self.peek() {
            Some(b'+') => {
                self.pos += 1;
                self.parse_factor()
            }
            Some(b'-') => {
                self.pos += 1;
                Ok(-self.parse_factor()?)
            }
            Some(b'(') => {
                self.pos += 1;
                let val = self.parse_expr()?;
                if self.peek() != Some(b')') {
                    return Err("unclosed '(' in angle expression".to_string());
                }
                self.pos += 1;
                Ok(val)
            }
            Some(_) => self.parse_atom(),
            None => Err("unexpected end of angle expression".to_string()),
        }
    }

    fn parse_atom(&mut self) -> Result<f64, String> {
        let start = self.pos;
        if self.bytes[start..].len() >= 2
            && self.bytes[start..start + 2].eq_ignore_ascii_case(b"pi")
            && !self
                .bytes
                .get(start + 2)
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
        {
            self.pos += 2;
            return Ok(PI);
        }
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == b'.' {
                self.pos += 1;
            } else if (c == b'e' || c == b'E') && self.pos > start {
                self.pos += 1;
                if matches!(self.peek(), Some(b'+' | b'-')) {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
        if self.pos == start {
            let tok = std::str::from_utf8(&self.bytes[start..]).unwrap_or("");
            return Err(format!("cannot parse '{}'", tok));
        }
        let tok = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| "invalid UTF-8 in angle".to_string())?;
        let val = tok
            .parse::<f64>()
            .map_err(|_| format!("cannot parse '{}'", tok))?;
        if !val.is_finite() {
            return Err(format!("non-finite numeric literal '{}'", tok));
        }
        Ok(val)
    }
}

fn eval_angle_expr(expr: &str) -> Result<f64, String> {
    let cleaned: String = expr.chars().filter(|c| !c.is_whitespace()).collect();
    if cleaned.is_empty() {
        return Err("empty angle expression".to_string());
    }
    let mut parser = AngleParser::new(&cleaned);
    let val = parser.parse_expr()?;
    if parser.pos != cleaned.len() {
        return Err(format!(
            "unexpected trailing '{}' in angle expression",
            &cleaned[parser.pos..]
        ));
    }
    if !val.is_finite() {
        return Err("non-finite angle expression".to_string());
    }
    Ok(val)
}

/// Pure n-qubit statevector `|psi>` of length `2^n` (qubit `0` = least significant bit `1 << 0`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Statevector {
    pub num_qubits: usize,
    pub amps: Vec<Complex64>,
    pub last_measurements: BTreeMap<usize, u8>,
}

impl Statevector {
    pub fn zero_state(num_qubits: usize) -> Self {
        let n = num_qubits.clamp(1, 12);
        let dim = 1usize << n;
        let mut amps = vec![Complex64::ZERO; dim];
        amps[0] = Complex64::ONE;
        Self {
            num_qubits: n,
            amps,
            last_measurements: BTreeMap::new(),
        }
    }

    pub fn dim(&self) -> usize {
        self.amps.len()
    }

    pub fn probabilities(&self) -> Vec<f64> {
        self.amps.iter().map(|c| c.norm_sqr()).collect()
    }

    pub fn apply_single_qubit_matrix(&mut self, target: usize, u: [[Complex64; 2]; 2]) {
        assert!(target < self.num_qubits);
        let bit = 1usize << target;
        let dim = self.dim();
        for i in 0..dim {
            if (i & bit) == 0 {
                let j = i | bit;
                let a0 = self.amps[i];
                let a1 = self.amps[j];
                self.amps[i] = u[0][0] * a0 + u[0][1] * a1;
                self.amps[j] = u[1][0] * a0 + u[1][1] * a1;
            }
        }
    }

    pub fn apply_op(&mut self, op: &GateOp, rng: &mut DeterministicRng) {
        let s = FRAC_1_SQRT_2;
        match *op {
            GateOp::Id(q) => {
                assert!(q < self.num_qubits);
            }
            GateOp::H(q) => self.apply_single_qubit_matrix(
                q,
                [
                    [Complex64::new(s, 0.0), Complex64::new(s, 0.0)],
                    [Complex64::new(s, 0.0), Complex64::new(-s, 0.0)],
                ],
            ),
            GateOp::X(q) => self.apply_single_qubit_matrix(
                q,
                [
                    [Complex64::ZERO, Complex64::ONE],
                    [Complex64::ONE, Complex64::ZERO],
                ],
            ),
            GateOp::Y(q) => self.apply_single_qubit_matrix(
                q,
                [
                    [Complex64::ZERO, Complex64::new(0.0, -1.0)],
                    [Complex64::new(0.0, 1.0), Complex64::ZERO],
                ],
            ),
            GateOp::Z(q) => self.apply_single_qubit_matrix(
                q,
                [
                    [Complex64::ONE, Complex64::ZERO],
                    [Complex64::ZERO, Complex64::new(-1.0, 0.0)],
                ],
            ),
            GateOp::S(q) => self.apply_single_qubit_matrix(
                q,
                [
                    [Complex64::ONE, Complex64::ZERO],
                    [Complex64::ZERO, Complex64::I],
                ],
            ),
            GateOp::Sdg(q) => self.apply_single_qubit_matrix(
                q,
                [
                    [Complex64::ONE, Complex64::ZERO],
                    [Complex64::ZERO, Complex64::new(0.0, -1.0)],
                ],
            ),
            GateOp::T(q) => self.apply_single_qubit_matrix(
                q,
                [
                    [Complex64::ONE, Complex64::ZERO],
                    [Complex64::ZERO, Complex64::cis(PI / 4.0)],
                ],
            ),
            GateOp::Tdg(q) => self.apply_single_qubit_matrix(
                q,
                [
                    [Complex64::ONE, Complex64::ZERO],
                    [Complex64::ZERO, Complex64::cis(-PI / 4.0)],
                ],
            ),
            GateOp::Rx { target, theta } => {
                let c = (0.5 * theta).cos();
                let sn = (0.5 * theta).sin();
                self.apply_single_qubit_matrix(
                    target,
                    [
                        [Complex64::new(c, 0.0), Complex64::new(0.0, -sn)],
                        [Complex64::new(0.0, -sn), Complex64::new(c, 0.0)],
                    ],
                );
            }
            GateOp::Ry { target, theta } => {
                let c = (0.5 * theta).cos();
                let sn = (0.5 * theta).sin();
                self.apply_single_qubit_matrix(
                    target,
                    [
                        [Complex64::new(c, 0.0), Complex64::new(-sn, 0.0)],
                        [Complex64::new(sn, 0.0), Complex64::new(c, 0.0)],
                    ],
                );
            }
            GateOp::Rz { target, theta } => {
                self.apply_single_qubit_matrix(
                    target,
                    [
                        [Complex64::cis(-0.5 * theta), Complex64::ZERO],
                        [Complex64::ZERO, Complex64::cis(0.5 * theta)],
                    ],
                );
            }
            GateOp::Phase { target, lambda } => {
                self.apply_single_qubit_matrix(
                    target,
                    [
                        [Complex64::ONE, Complex64::ZERO],
                        [Complex64::ZERO, Complex64::cis(lambda)],
                    ],
                );
            }
            GateOp::Cnot { control, target } => {
                if control == target {
                    return;
                }
                let c_mask = 1usize << control;
                let t_mask = 1usize << target;
                for i in 0..self.dim() {
                    if (i & c_mask) != 0 && (i & t_mask) == 0 {
                        self.amps.swap(i, i | t_mask);
                    }
                }
            }
            GateOp::Cz { control, target } => {
                if control == target {
                    return;
                }
                let mask = (1usize << control) | (1usize << target);
                for i in 0..self.dim() {
                    if (i & mask) == mask {
                        self.amps[i] = -self.amps[i];
                    }
                }
            }
            GateOp::CPhase {
                control,
                target,
                lambda,
            } => {
                if control == target {
                    return;
                }
                let mask = (1usize << control) | (1usize << target);
                let rot = Complex64::cis(lambda);
                for i in 0..self.dim() {
                    if (i & mask) == mask {
                        self.amps[i] *= rot;
                    }
                }
            }
            GateOp::Swap(a, b) => {
                if a == b {
                    return;
                }
                let ma = 1usize << a;
                let mb = 1usize << b;
                for i in 0..self.dim() {
                    if (i & ma) == 0 && (i & mb) != 0 {
                        let j = (i | ma) & !mb;
                        self.amps.swap(i, j);
                    }
                }
            }
            GateOp::Toffoli { c1, c2, target } => {
                if c1 == target || c2 == target || c1 == c2 {
                    return;
                }
                let cmask = (1usize << c1) | (1usize << c2);
                let tmask = 1usize << target;
                for i in 0..self.dim() {
                    if (i & cmask) == cmask && (i & tmask) == 0 {
                        self.amps.swap(i, i | tmask);
                    }
                }
            }
            GateOp::Measure(q) => {
                let outcome = self.measure_qubit(q, rng);
                self.last_measurements.insert(q, outcome);
            }
        }
    }

    pub fn measure_qubit(&mut self, target: usize, rng: &mut DeterministicRng) -> u8 {
        let mask = 1usize << target;
        let mut p1 = 0.0_f64;
        for (idx, amp) in self.amps.iter().enumerate() {
            if (idx & mask) != 0 {
                p1 += amp.norm_sqr();
            }
        }
        let outcome_one = rng.next_f64() < p1;
        let norm = if outcome_one {
            p1.max(1e-15).sqrt()
        } else {
            (1.0 - p1).max(1e-15).sqrt()
        };
        for (idx, amp) in self.amps.iter_mut().enumerate() {
            let bit_one = (idx & mask) != 0;
            if bit_one == outcome_one {
                *amp = amp.scale(1.0 / norm);
            } else {
                *amp = Complex64::ZERO;
            }
        }
        u8::from(outcome_one)
    }

    /// Executes circuit up to `step_limit` operations (`None` = full circuit).
    pub fn execute_circuit(
        circuit: &QuantumCircuit,
        step_limit: Option<usize>,
        seed: u64,
    ) -> Self {
        let mut sv = Self::zero_state(circuit.num_qubits);
        let mut rng = DeterministicRng::from_seed(seed);
        let limit = step_limit.unwrap_or(circuit.ops.len()).min(circuit.ops.len());
        for op in &circuit.ops[..limit] {
            sv.apply_op(op, &mut rng);
        }
        sv
    }

    /// Samples `shots` projective measurements in the computational Z basis without mutating `self`.
    pub fn sample_counts(&self, shots: usize, seed: u64) -> BTreeMap<usize, usize> {
        let probs = self.probabilities();
        let mut cdf = Vec::with_capacity(probs.len());
        let mut acc = 0.0;
        for p in probs {
            acc += p;
            cdf.push(acc);
        }
        let mut rng = DeterministicRng::from_seed(seed);
        let mut counts = BTreeMap::new();
        for _ in 0..shots {
            let u = rng.next_f64();
            let idx = cdf.partition_point(|&c| c < u).min(self.dim() - 1);
            *counts.entry(idx).or_insert(0) += 1;
        }
        counts
    }

    /// Reduced 2x2 density matrix for `target` qubit after tracing out all other qubits.
    pub fn reduced_single_qubit_rho(&self, target: usize) -> CMatrix {
        assert!(target < self.num_qubits);
        let mask = 1usize << target;
        let mut rho = CMatrix::zeros(2);
        for i in 0..self.dim() {
            if (i & mask) == 0 {
                let j = i | mask;
                let a0 = self.amps[i];
                let a1 = self.amps[j];
                rho.add_at(0, 0, a0 * a0.conj());
                rho.add_at(0, 1, a0 * a1.conj());
                rho.add_at(1, 0, a1 * a0.conj());
                rho.add_at(1, 1, a1 * a1.conj());
            }
        }
        rho
    }

    pub fn bloch_vector(&self, target: usize) -> [f64; 3] {
        let rho = self.reduced_single_qubit_rho(target);
        rho_to_bloch(&rho)
    }

    /// Reduced density matrix of subsystem A consisting of qubits `0..split_k` (`1 <= split_k < num_qubits`).
    pub fn reduced_subsystem_rho(&self, split_k: usize) -> CMatrix {
        let k = split_k.clamp(1, self.num_qubits.saturating_sub(1).max(1));
        if self.num_qubits == 1 {
            return self.reduced_single_qubit_rho(0);
        }
        let dim_a = 1usize << k;
        let dim_b = 1usize << (self.num_qubits - k);
        let mask_a = dim_a - 1;
        let mut rho_a = CMatrix::zeros(dim_a);
        for b in 0..dim_b {
            let b_offset = b << k;
            for ia in 0..dim_a {
                let amp_i = self.amps[b_offset | (ia & mask_a)];
                if amp_i.norm_sqr() == 0.0 {
                    continue;
                }
                for ja in 0..dim_a {
                    let amp_j = self.amps[b_offset | (ja & mask_a)];
                    rho_a.add_at(ia, ja, amp_i * amp_j.conj());
                }
            }
        }
        rho_a
    }

    /// Bipartite von Neumann entanglement entropy S(rho_A) in ebits across cut `0..split_k | split_k..n`.
    pub fn bipartite_entanglement_ebits(&self, split_k: usize) -> f64 {
        if self.num_qubits < 2 {
            return 0.0;
        }
        self.reduced_subsystem_rho(split_k).von_neumann_entropy_bits()
    }

    /// Full pure-state density matrix `|psi><psi|` for up to 4 qubits (`dim <= 16`).
    pub fn full_density_matrix(&self) -> Option<CMatrix> {
        if self.num_qubits > 4 {
            return None;
        }
        let d = self.dim();
        let mut rho = CMatrix::zeros(d);
        for r in 0..d {
            let ar = self.amps[r];
            for c in 0..d {
                rho.set(r, c, ar * self.amps[c].conj());
            }
        }
        Some(rho)
    }
}

/// Quantum Teleportation & noisy EPR channel simulator (Bennett et al. 1993).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TeleportationReport {
    pub theta: f64,
    pub phi: f64,
    pub channel_depolarizing_p: f64,
    pub alice_bell_bits: (u8, u8),
    pub source_bloch: [f64; 3],
    pub bob_bloch: [f64; 3],
    pub state_fidelity: f64,
}

pub fn simulate_teleportation(
    theta: f64,
    phi: f64,
    channel_depolarizing_p: f64,
    seed: u64,
) -> TeleportationReport {
    // Source pure state |psi> = cos(theta/2)|0> + e^{i phi} sin(theta/2)|1>
    let source_bloch = [
        theta.sin() * phi.cos(),
        theta.sin() * phi.sin(),
        theta.cos(),
    ];

    // Coherent 3-qubit teleportation run on q0 (payload), q1 (Alice EPR), q2 (Bob EPR)
    let mut sv = Statevector::zero_state(3);
    let mut rng = DeterministicRng::from_seed(seed);

    // Prepare payload on q0
    sv.apply_op(&GateOp::Ry { target: 0, theta }, &mut rng);
    sv.apply_op(&GateOp::Rz { target: 0, theta: phi }, &mut rng);
    // Distribute EPR pair on (q1, q2)
    sv.apply_op(&GateOp::H(1), &mut rng);
    sv.apply_op(&GateOp::Cnot { control: 1, target: 2 }, &mut rng);

    // Optional stochastic Pauli error on Bob's flying EPR qubit (depolarizing probability p)
    let p = channel_depolarizing_p.clamp(0.0, 1.0);
    let u_noise = rng.next_f64();
    if u_noise < p / 3.0 {
        sv.apply_op(&GateOp::X(2), &mut rng);
    } else if u_noise < 2.0 * p / 3.0 {
        sv.apply_op(&GateOp::Y(2), &mut rng);
    } else if u_noise < p {
        sv.apply_op(&GateOp::Z(2), &mut rng);
    }

    // Alice Bell-basis measurement on (q0, q1)
    sv.apply_op(&GateOp::Cnot { control: 0, target: 1 }, &mut rng);
    sv.apply_op(&GateOp::H(0), &mut rng);
    let m0 = sv.measure_qubit(0, &mut rng);
    let m1 = sv.measure_qubit(1, &mut rng);

    // Classical 2-bit feed-forward correction on Bob's qubit q2
    if m1 == 1 {
        sv.apply_op(&GateOp::X(2), &mut rng);
    }
    if m0 == 1 {
        sv.apply_op(&GateOp::Z(2), &mut rng);
    }

    // Ensemble average Bob Bloch vector under depolarizing noise p shrinks isotropically by (1 - p),
    // while single-shot coherent Bloch vector reflects exact Pauli trajectory; report analytical ensemble
    // fidelity F = 1 - 2p/3 alongside single-shot conditional readout if p == 0.
    let bob_bloch = if p <= 1e-12 {
        sv.bloch_vector(2)
    } else {
        [
            (1.0 - p) * source_bloch[0],
            (1.0 - p) * source_bloch[1],
            (1.0 - p) * source_bloch[2],
        ]
    };
    let dot = source_bloch[0] * bob_bloch[0]
        + source_bloch[1] * bob_bloch[1]
        + source_bloch[2] * bob_bloch[2];
    let state_fidelity = (0.5 * (1.0 + dot)).clamp(0.0, 1.0);

    TeleportationReport {
        theta,
        phi,
        channel_depolarizing_p: p,
        alice_bell_bits: (m0, m1),
        source_bloch,
        bob_bloch,
        state_fidelity,
    }
}
