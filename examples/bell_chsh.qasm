OPENQASM 2.0;
include "qelib1.inc";
// Bell state |Phi+> = (|00> + |11>) / sqrt(2) carrying 1.000 ebit of entanglement
qreg q[2];
creg c[2];

h q[0];
cx q[0],q[1];
