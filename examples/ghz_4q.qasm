OPENQASM 2.0;
include "qelib1.inc";
// 4-qubit Greenberger-Horne-Zeilinger state (|0000> + |1111>) / sqrt(2)
qreg q[4];
creg c[4];

h q[0];
cx q[0],q[1];
cx q[0],q[2];
cx q[0],q[3];
