OPENQASM 2.0;
include "qelib1.inc";
// 3-qubit Grover search marking |111> via CCZ (H-CCX-H) + 2-iteration diffusion
qreg q[3];
creg c[3];

h q[0];
h q[1];
h q[2];

// Iteration 1: Phase oracle marking |111>
h q[2];
ccx q[0],q[1],q[2];
h q[2];

// Diffusion operator 2|s><s| - I
h q[0];
h q[1];
h q[2];
x q[0];
x q[1];
x q[2];
h q[2];
ccx q[0],q[1],q[2];
h q[2];
x q[0];
x q[1];
x q[2];
h q[0];
h q[1];
h q[2];

// Iteration 2: Phase oracle marking |111>
h q[2];
ccx q[0],q[1],q[2];
h q[2];

// Diffusion operator 2|s><s| - I
h q[0];
h q[1];
h q[2];
x q[0];
x q[1];
x q[2];
h q[2];
ccx q[0],q[1],q[2];
h q[2];
x q[0];
x q[1];
x q[2];
h q[0];
h q[1];
h q[2];
