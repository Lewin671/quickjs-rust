// Derived from: test/built-ins/Function/prototype/apply/S15.3.4.3_A3_T1.js
function count() {
  return arguments.length;
}

if (count.apply(null, undefined) !== 0) { throw; }
if (count.apply(null, null) !== 0) { throw; }
