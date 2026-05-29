// Derived from: test/built-ins/Function/S15.3.5_A1_T2.js
var f = Function();

if (typeof f !== "function") { throw; }
if (f.length !== 0) { throw; }
if (Object.prototype.toString.call(f) !== "[object Function]") { throw; }
