// Derived from: test/built-ins/Function/prototype/S15.3.3.1_A4.js
function C() {}
C.prototype = { value: 8 };

var instance = new C();
if (instance.value !== 8) { throw; }
