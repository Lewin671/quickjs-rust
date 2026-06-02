// Derived from: test/built-ins/Array/prototype/pop/call-with-boolean.js
if (Array.prototype.pop.call(false) !== undefined) {
  throw "expected pop on a boolean receiver to return undefined";
}
