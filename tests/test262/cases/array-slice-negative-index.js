// Derived from: test/built-ins/Array/prototype/slice/S15.4.4.10_A2.1_T1.js
var array = [0, 1, 2, 3, 4].slice(-3, -1);
if (array.length !== 2) { throw; }
if (array[0] !== 2) { throw; }
if (array[1] !== 3) { throw; }
