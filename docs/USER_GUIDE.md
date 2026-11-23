# QuantaCrypt Studio — User Guide (`v2.2.0`)

QuantaCrypt Studio is released under the **GNU General Public License v3.0 or later (`GPL-3.0-or-later`)** as part of the CRV Science open-source observatory network ([`https://crvscience.com/apps`](https://crvscience.com/apps)).

## 1. Getting Started & Interface Layout

1. **Launch GUI**:
   ```powershell
   cargo run --release
   ```
   Pass `--skip-hub` or `--module <slug>` (for example `--module c3-ml-kem-lattice` or `--module a3-qaoa-max-cut`) to jump directly into a module workspace on startup.
2. **Top Navigation & Quick Group Filter Bar**:
   - **Observatory Hub (23)**: Returns to the 23-module overview.
   - **Group Pills (`I`, `Q`, `D`, `K`, `C`, `A`)**: One-click curriculum group filtering into the Observatory Hub.
   - **Simulations Dropdown**: Features an inline live search box (`code`, `title`, `slug`, or `summary`) alongside grouped module entries.
   - **Transport Controls**: Play/Pause (`Space`), Reset Stream (`R`), Simulation Speed slider (`0.25x` to `4.00x`), SQLite Workset **Presets** (`F5`/`F9`), **Citations (`F1`)**, and **About**.

## 2. Keyboard & Mouse Shortcuts

All global single-key shortcuts honor `!ctx.wants_keyboard_input()`, so typing in the search filter box, SQLite slot name field, or QASM editor will never trigger module switches or resets.

| Input | Scope | Action |
| --- | --- | --- |
| `Left` / `Right` | Workspace / Hub | Switch to previous / next module across the 23-module sequence |
| `H` | Global | Toggle between Observatory Hub and active Module Workspace |
| `Space` | Workspace | Toggle continuous stochastic playback (`playing` state) |
| `R` | Workspace | Reset live arena counters, traces, and deterministic RNG stream |
| `F1` | Global | Toggle Shortcuts & Primary Literature Citations modal |
| `F5` / `F9` | Global | Quick-save / Quick-load workset state to SQLite slot `quick_save` |
| `Esc` | Global | Dismiss open modal (`About` -> `Citations` -> `Worksets`), then return to Hub |
| Drag on Bloch Sphere | `Q1`, `Q3`, `Q4`, `A5` | Orbit camera `(yaw, pitch)` freely in 3D |
| Double-Click Bloch Sphere | `Q1`, `Q3`, `Q4`, `A5` | Reset camera orientation to default `(yaw = 0.62, pitch = 0.34)` |
| Hover Density Heatmap | `Q2` | Highlight `rho[row, col]` cell with complex value, `|rho|`, and phase angle |
| Hover Time-Series Plot | Multi-module | Display crosshair lines, cursor `(x, y)`, and nearest traced sample |

## 3. Module Workflows & Regime Presets

### Interactive Gate & OpenQASM 2.0 Circuit Lab (`Q2`)
- Choose built-in presets (`Bell |Phi+>`, `4-Qubit GHZ`, `3-Qubit QFT`), resize qubit count (`1..=6`), step cursor (`|<`, `<`, `>`, `>|`), append single/two/three-qubit gates from the interactive palette, or paste custom OpenQASM 2.0 directly.
- Inspect the side-by-side computational basis amplitude bars, the complex density matrix heatmap (with phase colorwheel `arg(rho_ij)` and `|rho_ij|` bar), bipartite entanglement entropy `S(rho_A)`, and CHSH/fidelity readouts.

### Lattice PQC (`C3` ML-KEM & `C4` ML-DSA)
- **`C3` ML-KEM**: Click **`ML-KEM-512`**, **`ML-KEM-768`**, or **`High Noise (eta=18)`** presets to compare normal decapsulation noise against `q/4 = 832`, inspect FIPS 203 `eta_1 / eta_2` and `(d_u, d_v)` compression telemetry, and trigger Fujisaki-Okamoto implicit rejection when decryption margins fail.
- **`C4` ML-DSA**: Compare **`ML-DSA Safe`**, **`Tight Margin`**, and **`Wide Challenge`** regimes showing raw tail leakage in `y + c*s` versus accepted rejection-sampled signatures strictly inside `[-gamma_1 + beta, gamma_1 - beta]`.

### QKD (`K1` BB84) & Fault-Tolerant Computing (`A1`–`A4`)
- **`K1` BB84**: Switch between `Clean (0% Eve)`, `Edge (~11%)`, and `Attack (60% Eve)` to watch live sifted QBER approach and breach the `11.0%` Shor-Preskill security bound while Cascade parity leakage reduces net distilled secret key bits.
- **`A1` Surface Code**: Inspect boundary weight-2 half-plaquettes and interior weight-4 stabilizers across code distances `d in {3, 5, 7}` while crossing the `~1.0%` threshold (`Sub-Th 0.4%` vs `Over-Th 1.8%`).
- **`A3` QAOA Max-Cut**: Track the 8-atom Rydberg spin graph partition and live `C=0..10` sampled cut histogram (`C_max = 10` for optimal alternating partitions such as `0b0101_1010`).

## 4. SQLite WAL Worksets & CLI Batch Workflows

- Open **Presets** to manage named scenarios inside `quantacrypt_worksets.sqlite3` or export/import SHA-256 tamper-checked `.qworkset.json` envelopes.
- Use headless CLI commands in scripts/CI (`run-qasm`, `simulate-qkd`, `shor-factor`, `grover-search`, `ml-kem-demo`, `qudit-bench`, or `--headless-screenshot <path> --screenshot-view 1..4`) without opening a GUI window.
