// Derived from: test/built-ins/Array/prototype/concat/S15.4.4.4_A1_T1.js
var array = [].concat([0, 1], [2, 3, 4]);
if (array.length !== 5) { throw; }
if (array[0] !== 0) { throw; }
if (array[4] !== 4) { throw; }
