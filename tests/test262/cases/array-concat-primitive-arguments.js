// Derived from: test/built-ins/Array/prototype/concat/S15.4.4.4_A2_T1.js
var array = [0].concat("x", true, null);
if (array.length !== 4) { throw; }
if (array[1] !== "x") { throw; }
if (array[2] !== true) { throw; }
if (array[3] !== null) { throw; }
