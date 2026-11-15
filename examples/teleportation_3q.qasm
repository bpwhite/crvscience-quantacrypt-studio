OPENQASM 2.0;
include "qelib1.inc";
// 3-qubit Quantum Teleportation state preparation + Bell analyzer (q0 -> q2 via EPR q1,q2)
qreg q[3];
creg c[3];

// Prepare non-trivial payload state on q[0]
ry(1.15) q[0];
rz(0.85) q[0];

// Distribute Bell pair |Phi+> across q[1] and q[2]
h q[1];
cx q[1],q[2];

// Alice Bell-basis rotation on (q[0], q[1])
cx q[0],q[1];
h q[0];

// Coherent deferred-measurement feed-forward equivalent prior to Z-basis readout
cx q[1],q[2];
cz q[0],q[2];
