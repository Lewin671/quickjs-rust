// Derived from: test/built-ins/Function/prototype/S15.3.3.1_A1.js
function f() {}

if (Object.getPrototypeOf(f) !== Function.prototype) { throw; }
if (!Function.prototype.isPrototypeOf(f)) { throw; }
if (Function.prototype.constructor !== Function) { throw; }
