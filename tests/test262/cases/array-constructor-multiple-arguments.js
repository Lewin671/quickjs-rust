// Derived from: test/built-ins/Array/S15.4.2.1_A2.1_T1.js
var array = Array(1, 2, "three");
if (array.length !== 3) { throw; }
if (array[0] !== 1) { throw; }
if (array[2] !== "three") { throw; }
