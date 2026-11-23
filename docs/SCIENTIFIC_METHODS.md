# QuantaCrypt Studio — Scientific & Cryptographic Methods (`v2.2.0`)

QuantaCrypt Studio (`GPL-3.0-or-later`) implements deterministic, reproducible physics and post-quantum cryptography simulations using exact finite-dimensional linear algebra, continuous `dt`-driven Monte-Carlo trajectory sampling, and NIST FIPS-aligned cryptographic algorithms.

---

## 1. Foundational Interferometry & Open-System Quantum Mechanics (`I1`–`I4`, `Q1`–`Q5`)

1. **Double-Slit Complementarity (`I1`)**:
   Single-particle detection intensity combines coherent two-path amplitudes weighted by path distinguishability `D in [0, 1]` and Gaussian envelope width `sigma_env`:

       I(x) = exp(-x^2 / (2 * sigma_env^2)) * [1 + V * cos(k * d * x)],   V = sqrt(1 - D^2)

   satisfying the Englert-Greenberger duality inequality `D^2 + V^2 <= 1`.

2. **Quantum Statevectors, Density Matrices & Entanglement (`Q2`)**:
   Pure `n`-qubit states evolve under unitary gate operators `U`, yielding reduced density operator `rho_A = Tr_B(|psi><psi|)` across bipartite cut `k | (n - k)`. Entanglement entropy is evaluated in bits from eigenvalues `lambda_i` of `rho_A`:

       S(rho_A) = - sum_i lambda_i * log2(lambda_i)   (in ebits)

3. **Kraus Decoherence & 3-Qubit Stabilizer Recovery (`Q4`)**:
   Open-system states evolve under trace-preserving Kraus maps `E(rho) = sum_k E_k rho E_k^dagger` (`sum_k E_k^dagger E_k = I`) across bit-flip (`X`), amplitude-damping (`T_1`), phase-damping (`T_2`), and isotropic depolarizing channels. Three-qubit repetition codes detect single faults with syndrome weight parity and reduce failure probability from `p` to `3p^2 - 2p^3` below `p = 0.5`.

---

## 2. High-Dimensional Qudits & Quantum Key Distribution (`D1`–`D2`, `K1`–`K3`)

1. **Weyl-Heisenberg Operator Algebra (`D1`, `D2`)**:
   On Hilbert dimension `d in 2..=8` with `omega = exp(2 * pi * i / d)`, shift `X_d |j> = |(j + 1) mod d>` and clock `Z_d |j> = omega^j |j>` satisfy:

       Z_d * X_d = omega * X_d * Z_d

   Isotropic depolarizing teleportation fidelity follows `F_tel(p) = 1 - p * (d - 1) / d`, beating the classical measure-and-prepare bound `F_class = 2 / (d + 1)`.

2. **BB84 Cascade Reconciliation & Privacy Amplification (`K1`)**:
   With binary Shannon entropy `h2(q) = -q * log2(q) - (1 - q) * log2(1 - q)`, intercept-resend eavesdropping rate `p_Eve` and fiber flip noise `p_ch` produce observed error rate:

       QBER = (0.25 * p_Eve) * (1 - p_ch) + (1 - 0.25 * p_Eve) * p_ch

   Channels remain secure below the Shor-Preskill bound `QBER < 11.0%` (`1 - 2 * h2(QBER) > 0`). Multi-pass binary Cascade reconciliation tracks total parity bits disclosed on the public classical channel `leak_EC = leaked_parity_bits`. Universal-2 Toeplitz hashing shrinks the reconciled key of length `L_sift` strictly within the residual min-entropy budget:

       L_final <= floor(L_sift - leak_EC - ceil(L_sift * h2(QBER)) - delta_sec)

   guaranteeing `L_final <= L_sift - leak_EC` even in small-block or elevated-QBER regimes.

---

## 3. Quantum Cryptanalysis & NIST Post-Quantum Cryptography (`C1`–`C4`)

1. **Shor Period Finding (`C1`) & Grover Amplitude Search (`C2`)**:
   Shor finds multiplicative order `r` where `a^r = 1 (mod N)` via modular-exponentiation interference and extracts factors `gcd(a^(r/2) +/- 1, N)`. Grover's diffusion operator amplifies a marked `n`-bit key (`N = 2^n`) to peak probability after `k_opt = floor((pi / 4) * sqrt(N))` oracle calls.

2. **NIST FIPS 203 ML-KEM over Negacyclic Ring `R_q` (`C3`)**:
   Arithmetic runs in `R_q = Z_q[X] / (X^n + 1)` with `q = 3329`. Key steps match FIPS 203 Table 2:
   - Matrix `A in R_q^{k x k}` sampled without modulo bias by `u32` rejection sampling (`draw < floor(2^32 / q) * q`).
   - Centered binomial distributions `CBD_{eta_1}` for `(s, e, r)` and `CBD_{eta_2}` for `(e_1, e_2)`, distinguishing `ML-KEM-512` (`k = 2, eta_1 = 3, eta_2 = 2, d_u = 10, d_v = 4`), `ML-KEM-768` (`k = 3, eta_1 = 2, eta_2 = 2, d_u = 10, d_v = 4`), and `ML-KEM-1024` (`k = 4, eta_1 = 2, eta_2 = 2, d_u = 11, d_v = 5`).
   - Lossy ciphertext compression / decompression:

         Compress_d(x)   = round((2^d / q) * x) mod 2^d
         Decompress_d(y) = round((q / 2^d) * y)

   - Decapsulation computes `w = Decompress_{d_v}(v) - s^T Decompress_{d_u}(u)` and tests Fujisaki-Okamoto (FO) re-encapsulation `(u', v') == (u, v)`, triggering pseudorandom implicit rejection `K_bar` on mismatch or when total decryption noise `|e_dec| > floor(q / 4) = 832`.

3. **NIST FIPS 204 ML-DSA Fiat-Shamir with Aborts (`C4`)**:
   Uniform masking `y in [-gamma_1 + 1, gamma_1]^n` is combined with a signed sparse challenge `c in {-1, 0, +1}^n` of exact Hamming weight `tau` via negacyclic product `z = y + c * s mod (X^n + 1)`. Rejection aborts whenever `||z||_inf >= gamma_1 - beta` (`beta = tau * eta`), ensuring accepted signatures are statistically independent of secret `s`.

---

## 4. Fault-Tolerant & Variational Quantum Algorithms (`A1`–`A5`)

- **Rotated Surface Code (`A1`)**: `[[d^2, 1, d]]` planar patch with `(d - 1)^2` bulk weight-4 plaquettes and `2*(d - 1)` boundary weight-2 half-plaquettes yielding `d^2 - 1` commuting stabilizers (`1` logical qubit), with logical failure scaling `P_L(d) ~ 0.08 * (p / p_th)^((d + 1) / 2)` around `p_th ~ 1.0%`.
- **QAOA Max-Cut (`A3`)**: Evaluates the 8-node cubic/ring-chord Rydberg graph (`12` edges, `C_max = 10`, e.g., `0b0101_1010`) across full cut spectrum bins `C in 0..=10`.
- **Bravyi-Kitaev `15-to-1` Magic-State Factory (`A4`, `A5`)**: Distills 15 noisy `|T>` states (`p_in`) under Reed-Muller `[15, 1, 3]` post-selection, suppressing output logical infidelity to `p_out ~ 35 * p_in^3` when post-selected.
