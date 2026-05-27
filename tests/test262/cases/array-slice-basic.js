// Derived from: test/built-ins/Array/prototype/slice/S15.4.4.10_A1.1_T1.js
var array = [0, 1, 2, 3, 4].slice(0, 3);
if (array.length !== 3) { throw; }
if (array[0] !== 0) { throw; }
if (array[2] !== 2) { throw; }
if (array[3] !== undefined) { throw; }
