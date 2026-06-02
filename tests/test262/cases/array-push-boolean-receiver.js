// Derived from: test/built-ins/Array/prototype/push/call-with-boolean.js
if (Array.prototype.push.call(true) !== 0) {
  throw "expected push on true receiver to return 0";
}
if (Array.prototype.push.call(false) !== 0) {
  throw "expected push on false receiver to return 0";
}
