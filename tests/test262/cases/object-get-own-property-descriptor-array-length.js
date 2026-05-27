// Derived from: test/built-ins/Array/S15.4.5.2_A1_T1.js
var desc = Object.getOwnPropertyDescriptor([1, 2], "length");
if (desc.value !== 2) { throw; }
if (desc.enumerable !== false) { throw; }
if (desc.writable !== true) { throw; }
if (desc.configurable !== false) { throw; }
