// Derived from: test/built-ins/String/prototype/toString/length.js
if (String.prototype.toString.length !== 0) {
  throw "expected String.prototype.toString.length to be 0";
}
if (String.prototype.valueOf.length !== 0) {
  throw "expected String.prototype.valueOf.length to be 0";
}
