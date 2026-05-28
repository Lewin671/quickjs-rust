// Derived from: test/built-ins/Function/prototype/apply/length.js
function f() {}

if (f.apply.length !== 2) { throw; }
if (f.apply.propertyIsEnumerable("length")) { throw; }
