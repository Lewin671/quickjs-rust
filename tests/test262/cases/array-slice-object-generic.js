// Derived from: test/built-ins/Array/prototype/slice/S15.4.4.10_A2_T1.js
var object = { length: 5, 0: 0, 1: 1, 2: 2, 3: 3, 4: 4 };
object.slice = Array.prototype.slice;
var result = object.slice(0, 3);
if (!Array.isArray(result)) { throw; }
if (result.length !== 3) { throw; }
if (result[0] !== 0 || result[1] !== 1 || result[2] !== 2) { throw; }
