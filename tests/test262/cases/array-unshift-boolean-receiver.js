// Derived from: test/built-ins/Array/prototype/unshift/call-with-boolean.js
if (Array.prototype.unshift.call(true) !== 0) {
  throw "expected unshift called on true to return zero";
}
if (Array.prototype.unshift.call(false) !== 0) {
  throw "expected unshift called on false to return zero";
}
