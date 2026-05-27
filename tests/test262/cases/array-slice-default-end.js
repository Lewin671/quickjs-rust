// Derived from: test/built-ins/Array/prototype/slice/S15.4.4.10_A1.2_T1.js
var array = [0, 1, 2, 3, 4].slice(2);
if (array.length !== 3) { throw; }
if (array[0] !== 2) { throw; }
if (array[2] !== 4) { throw; }
