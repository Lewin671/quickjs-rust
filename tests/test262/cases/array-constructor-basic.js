// Derived from: test/built-ins/Array/S15.4.2.1_A1.1_T1.js
if (typeof Array !== "function") { throw; }
if (Array.length !== 1) { throw; }
if (Array().length !== 0) { throw; }
if (Array("value")[0] !== "value") { throw; }
