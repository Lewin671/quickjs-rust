// Derived from: test/built-ins/Array/prototype/toString/S15.4.4.2_A1_T2.js
var array = [1, "x", true];
if (array.toString() !== array.join()) { throw; }
if (array.toString() !== "1,x,true") { throw; }
