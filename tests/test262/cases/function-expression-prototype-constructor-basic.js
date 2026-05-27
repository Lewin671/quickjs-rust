// Derived from: test/built-ins/Function/prototype/constructor/S15.3.4.1_A1_T1.js
var C = function Named() {};

if (C.prototype.constructor !== C) { throw; }
