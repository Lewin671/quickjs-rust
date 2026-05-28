// Derived from: test/built-ins/Function/prototype/call/S15.3.4.4_A13.js
function f() {}

if (f.call.length !== 1) { throw; }
if (f.call.propertyIsEnumerable("length")) { throw; }
