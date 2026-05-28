// Derived from: test/built-ins/Function/prototype/call/S15.3.4.4_A3_T1.js
function getThis() {
  return this;
}

if (getThis.call(undefined) !== this) { throw; }
if (getThis.call(null) !== this) { throw; }
