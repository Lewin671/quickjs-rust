// Derived from: test/language/literals/numeric/S7.8.3_A3.4_T1.js
if (1.1e1 !== 11) { throw; }
if (1.1E1 !== 11) { throw; }
if (1.1e-1 !== 0.11) { throw; }
if (.1e0 !== 0.1) { throw; }
