// Derived from: test/built-ins/Array/prototype/concat/S15.4.4.4_A3_T1.js
var array = [0].concat([1, 2], 3, [4]);
if (array.length !== 5) { throw; }
if (array[2] !== 2) { throw; }
if (array[3] !== 3) { throw; }
