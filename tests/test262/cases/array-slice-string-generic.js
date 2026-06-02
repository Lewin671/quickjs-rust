// Derived from: test/built-ins/Array/prototype/slice/S15.4.4.10_A2_T5.js
var result = Array.prototype.slice.call("abc", 1, 3);
if (!Array.isArray(result)) { throw; }
if (result.length !== 2) { throw; }
if (result[0] !== "b" || result[1] !== "c") { throw; }
