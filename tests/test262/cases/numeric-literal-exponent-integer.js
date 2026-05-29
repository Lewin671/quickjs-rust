// Derived from: test/language/literals/numeric/S7.8.3_A4.2_T1.js
if (1e01 !== 10) { throw; }
if (1E01 !== 10) { throw; }
if (1e+01 !== 10) { throw; }
if (1E-01 !== 0.1) { throw; }
