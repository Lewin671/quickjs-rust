// Derived from: test/built-ins/Function/S15.3.5_A3_T1.js
function C() {}
C.prototype = { value: 1 };

var instance = new C();
if (instance.constructor !== Object) { throw; }
